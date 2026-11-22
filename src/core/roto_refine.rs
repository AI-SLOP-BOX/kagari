//! Human-friendly Rotoscoping Edge Refinement:
//! - **Color-Guided Matte Feather**: snaps rough brush/mask boundaries to actual image edges
//! - **Choke / Expand**: tightens or expands the matte boundary
//! - **Foreground Color Decontamination**: removes background fringe bleed on semi-transparent hair/edges

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RotoRefineParams {
    /// Edge-aware feather radius in pixels (`1..=8`, default `2`).
    pub feather_radius: u32,
    /// Edge color sensitivity (`0.05..=1.0`, lower = snaps tighter to image edges, default `0.18`).
    pub edge_sensitivity: f32,
    /// Matte choke/expand (`-0.5..=0.5`, positive shrinks matte, negative expands, default `0.0`).
    pub choke: f32,
    /// Foreground color decontamination strength (`0.0..=1.0`, default `0.8`).
    pub decontamination: f32,
}

impl Default for RotoRefineParams {
    fn default() -> Self {
        Self {
            feather_radius: 2,
            edge_sensitivity: 0.18,
            choke: 0.0,
            decontamination: 0.8,
        }
    }
}

/// Refines a rough `[0..1]` roto `matte` against linear `rgba` (`w*h*4`), returning
/// `(decontaminated_rgba, refined_matte)`.
pub fn refine_roto_matte(
    rgba: &[f32],
    matte: &[f32],
    width: u32,
    height: u32,
    params: &RotoRefineParams,
) -> Option<(Vec<f32>, Vec<f32>)> {
    let count = (width as usize).checked_mul(height as usize)?;
    if count == 0 || rgba.len() != count * 4 || matte.len() != count {
        return None;
    }

    let r = params.feather_radius.clamp(1, 8) as i32;
    let sigma_c = params.edge_sensitivity.max(0.02);
    let inv_two_sigma_c2 = 1.0 / (2.0 * sigma_c * sigma_c);
    let inv_two_sigma_s2 = 1.0 / (2.0 * (r as f32 * 0.6).max(0.5).powi(2));
    let w = width as i32;
    let h = height as i32;

    let mut refined_alpha = vec![0.0_f32; count];
    let mut out_rgba = rgba.to_vec();

    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            let c_idx = idx * 4;
            let cr = rgba[c_idx];
            let cg = rgba[c_idx + 1];
            let cb = rgba[c_idx + 2];

            let mut sum_a = 0.0_f32;
            let mut sum_w = 0.0_f32;
            let mut fg_r = 0.0_f32;
            let mut fg_g = 0.0_f32;
            let mut fg_b = 0.0_f32;
            let mut fg_w = 0.0_f32;

            for ky in -r..=r {
                let ny = (y + ky).clamp(0, h - 1);
                for kx in -r..=r {
                    let nx = (x + kx).clamp(0, w - 1);
                    let n_idx = (ny * w + nx) as usize;
                    let nc = n_idx * 4;
                    let dr = rgba[nc] - cr;
                    let dg = rgba[nc + 1] - cg;
                    let db = rgba[nc + 2] - cb;
                    let c_dist2 = dr * dr + dg * dg + db * db;
                    let s_dist2 = (kx * kx + ky * ky) as f32;
                    let wt = (-c_dist2 * inv_two_sigma_c2 - s_dist2 * inv_two_sigma_s2).exp();

                    let na = matte[n_idx].clamp(0.0, 1.0);
                    sum_a += na * wt;
                    sum_w += wt;

                    if na > 0.85 {
                        fg_r += rgba[nc] * wt;
                        fg_g += rgba[nc + 1] * wt;
                        fg_b += rgba[nc + 2] * wt;
                        fg_w += wt;
                    }
                }
            }

            let mut a = (sum_a / sum_w.max(1e-6)).clamp(0.0, 1.0);
            if params.choke.abs() > 1e-4 {
                a = (a - params.choke).clamp(0.0, 1.0);
            }
            refined_alpha[idx] = a;

            // Decontaminate semi-transparent edge pixels using nearby pure foreground color
            if a > 0.02 && a < 0.95 && fg_w > 1e-4 && params.decontamination > 1e-4 {
                let blend = (1.0 - a) * params.decontamination.clamp(0.0, 1.0);
                out_rgba[c_idx] = cr + (fg_r / fg_w - cr) * blend;
                out_rgba[c_idx + 1] = cg + (fg_g / fg_w - cg) * blend;
                out_rgba[c_idx + 2] = cb + (fg_b / fg_w - cb) * blend;
            }
            out_rgba[c_idx + 3] = a;
        }
    }

    Some((out_rgba, refined_alpha))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roto_refine_decontaminates_edge_and_respects_color_boundary() {
        // 4x1 strip: [red_fg, red_fg, green_edge_spill, blue_bg]
        let rgba = vec![
            0.9, 0.1, 0.1, 1.0,
            0.9, 0.1, 0.1, 1.0,
            0.2, 0.7, 0.2, 1.0,
            0.1, 0.1, 0.9, 1.0,
        ];
        let matte = vec![1.0, 1.0, 0.5, 0.0];
        let (out_rgba, out_alpha) = refine_roto_matte(
            &rgba,
            &matte,
            4,
            1,
            &RotoRefineParams {
                feather_radius: 1,
                edge_sensitivity: 0.25,
                choke: 0.0,
                decontamination: 1.0,
            },
        )
        .expect("refine");

        // Pure foreground stays opaque; pure background stays ~0
        assert!(out_alpha[0] > 0.95);
        assert!(out_alpha[3] < 0.15);
        // Decontaminated semi-transparent pixel shifts red channel up from 0.2 toward foreground red
        assert!(out_rgba[2 * 4] > 0.2);
    }
}
