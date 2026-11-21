use super::*;

/// Per-frame positions of the four tracked corner features.
///
/// Corner order matches AE: [top_left, top_right, bottom_right, bottom_left].
/// Positions after the first analysed frame are produced by single-step
/// tracking (the same convention as [`TrackerEngine::analyze_track`]).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct QuadTrackData {
    pub frames: Vec<u32>,
    pub corners: Vec<[[f32; 2]; 4]>,
}

/// Solves the 3×3 planar homography H mapping `from` → `to` via the Direct
/// Linear Transform with h33 normalised to 1. Returns None when the point
/// configuration is degenerate (duplicates or collinear corners).
pub fn compute_homography(from: [[f32; 2]; 4], to: [[f32; 2]; 4]) -> Option<[[f64; 3]; 3]> {
    let mut a = [[0.0f64; 8]; 8];
    let mut b = [0.0f64; 8];
    for i in 0..4 {
        let [x, y] = [from[i][0] as f64, from[i][1] as f64];
        let [u, v] = [to[i][0] as f64, to[i][1] as f64];
        let r1 = i * 2;
        let r2 = r1 + 1;
        a[r1] = [-x, -y, -1.0, 0.0, 0.0, 0.0, u * x, u * y];
        b[r1] = -u;
        a[r2] = [0.0, 0.0, 0.0, -x, -y, -1.0, v * x, v * y];
        b[r2] = -v;
    }

    // Gaussian elimination with partial pivoting.
    for col in 0..8 {
        let pivot_row = (col..8)
            .max_by(|&l, &r| {
                a[l][col]
                    .abs()
                    .partial_cmp(&a[r][col].abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(col);
        if a[pivot_row][col].abs() < 1e-10 {
            return None;
        }
        a.swap(col, pivot_row);
        b.swap(col, pivot_row);
        for row in (col + 1)..8 {
            let factor = a[row][col] / a[col][col];
            let pivot = a[col]; // owned copy: avoids aliasing borrows below
            for (ac, bc) in a[row][col..].iter_mut().zip(pivot[col..].iter()) {
                *ac -= factor * bc;
            }
            b[row] -= factor * b[col];
        }
    }

    // Back substitution.
    let mut h = [0.0f64; 8];
    for row in (0..8).rev() {
        let sum: f64 = a[row][row + 1..]
            .iter()
            .zip(h[row + 1..].iter())
            .map(|(&av, &hv)| av * hv)
            .sum();
        h[row] = (b[row] - sum) / a[row][row];
    }

    let result = [[h[0], h[1], h[2]], [h[3], h[4], h[5]], [h[6], h[7], 1.0]];

    // Residual validation: the linear system can be solvable even when no
    // true homography exists (e.g. two source corners mapped to one target).
    // Reject any solution that does not reproduce the correspondences.
    for (f, t) in from.iter().zip(to.iter()) {
        let m = apply_homography(&result, *f);
        if !m[0].is_finite()
            || !m[1].is_finite()
            || (m[0] - t[0]).abs() > 1e-2
            || (m[1] - t[1]).abs() > 1e-2
        {
            return None;
        }
    }

    Some(result)
}

/// Applies a 3×3 homography to a 2D point. Points mapping through w≈0
/// (infinity) return f32::MAX components.
pub fn apply_homography(h: &[[f64; 3]; 3], p: [f32; 2]) -> [f32; 2] {
    let [x, y] = [p[0] as f64, p[1] as f64];
    if !x.is_finite() || !y.is_finite() || h.iter().flatten().any(|value| !value.is_finite()) {
        return [f32::MAX, f32::MAX];
    }
    let w = h[2][0] * x + h[2][1] * y + h[2][2];
    if !w.is_finite() || w.abs() < 1e-12 {
        return [f32::MAX, f32::MAX];
    }
    let mapped_x = (h[0][0] * x + h[0][1] * y + h[0][2]) / w;
    let mapped_y = (h[1][0] * x + h[1][1] * y + h[1][2]) / w;
    if !mapped_x.is_finite() || !mapped_y.is_finite() {
        return [f32::MAX, f32::MAX];
    }
    [
        mapped_x.clamp(f64::from(f32::MIN), f64::from(f32::MAX)) as f32,
        mapped_y.clamp(f64::from(f32::MIN), f64::from(f32::MAX)) as f32,
    ]
}

impl TrackerEngine {
    /// Tracks four corner features over a frame range for perspective
    /// corner-pin workflows. Every referenced tracker index must exist on the
    /// layer, otherwise the result is empty.
    pub fn analyze_quad_track(
        comp: &Composition,
        layer_idx: usize,
        tracker_indices: [usize; 4],
        start_frame: u32,
        end_frame: u32,
    ) -> QuadTrackData {
        Self::analyze_quad_track_cancellable(
            comp,
            layer_idx,
            tracker_indices,
            start_frame,
            end_frame,
            None,
        )
    }

    pub fn analyze_quad_track_cancellable(
        comp: &Composition,
        layer_idx: usize,
        tracker_indices: [usize; 4],
        start_frame: u32,
        end_frame: u32,
        cancel_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    ) -> QuadTrackData {
        let mut out = QuadTrackData::default();
        if end_frame < start_frame {
            return out;
        }
        let Some(layer) = comp.layers.get(layer_idx) else {
            return out;
        };
        if tracker_indices.iter().any(|&i| i >= layer.trackers.len()) {
            return out;
        }
        let fps = comp.fps;
        for f in start_frame..=end_frame {
            if cancel_flag
                .as_ref()
                .is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Relaxed))
            {
                return QuadTrackData::default();
            }
            let mut frame_corners = [[0.0f32; 2]; 4];
            for (slot, &ti) in tracker_indices.iter().enumerate() {
                frame_corners[slot] = Self::track_next_frame(layer, fps, ti, f.saturating_sub(1))
                    .unwrap_or_else(|| layer.trackers[ti].position.evaluate(f));
            }
            out.frames.push(f);
            out.corners.push(frame_corners);
        }
        out
    }
}

/// Computes per-frame homographies mapping `source_rect` onto each tracked
/// quad. Degenerate frames yield None entries.
pub fn quad_homographies(
    track: &QuadTrackData,
    source_rect: [[f32; 2]; 4],
) -> Vec<Option<[[f64; 3]; 3]>> {
    track
        .corners
        .iter()
        .map(|quad| compute_homography(source_rect, *quad))
        .collect()
}

/// Removes single-frame position spikes from a tracked quad. A corner sample
/// that jumps more than `max_jump` pixels from BOTH neighbours is replaced by
/// their midpoint. Motion-free tracks pass through unchanged.
pub fn smooth_quad_track(track: &QuadTrackData, max_jump: f32) -> QuadTrackData {
    let n = track.frames.len();
    if n < 3 || max_jump <= 0.0 {
        return track.clone();
    }
    let mut out = track.clone();
    for corner in 0..4 {
        for i in 1..n - 1 {
            let prev = track.corners[i - 1][corner];
            let cur = track.corners[i][corner];
            let next = track.corners[i + 1][corner];
            let d_prev = (cur[0] - prev[0]).hypot(cur[1] - prev[1]);
            let d_next = (next[0] - cur[0]).hypot(next[1] - cur[1]);
            if d_prev > max_jump && d_next > max_jump {
                out.corners[i][corner] = [(prev[0] + next[0]) * 0.5, (prev[1] + next[1]) * 0.5];
            }
        }
    }
    out
}

/// Bakes 4-point perspective quad tracking keyframes directly into a layer's CornerPin effect.
pub fn apply_quad_track_to_corner_pin(layer: &mut Layer, track: &QuadTrackData) -> bool {
    if track.frames.is_empty() || track.corners.is_empty() {
        return false;
    }

    let mut tl_kfs = Vec::with_capacity(track.frames.len());
    let mut tr_kfs = Vec::with_capacity(track.frames.len());
    let mut br_kfs = Vec::with_capacity(track.frames.len());
    let mut bl_kfs = Vec::with_capacity(track.frames.len());

    for (i, &f) in track.frames.iter().enumerate() {
        if let Some(corners) = track.corners.get(i) {
            tl_kfs.push(Keyframe::new(f, corners[0], InterpolationType::Linear));
            tr_kfs.push(Keyframe::new(f, corners[1], InterpolationType::Linear));
            br_kfs.push(Keyframe::new(f, corners[2], InterpolationType::Linear));
            bl_kfs.push(Keyframe::new(f, corners[3], InterpolationType::Linear));
        }
    }

    let top_left = if tl_kfs.len() == 1 {
        Animatable::Constant(tl_kfs[0].value)
    } else {
        Animatable::Animated(tl_kfs)
    };
    let top_right = if tr_kfs.len() == 1 {
        Animatable::Constant(tr_kfs[0].value)
    } else {
        Animatable::Animated(tr_kfs)
    };
    let bottom_right = if br_kfs.len() == 1 {
        Animatable::Constant(br_kfs[0].value)
    } else {
        Animatable::Animated(br_kfs)
    };
    let bottom_left = if bl_kfs.len() == 1 {
        Animatable::Constant(bl_kfs[0].value)
    } else {
        Animatable::Animated(bl_kfs)
    };

    let corner_pin_type = crate::core::timeline::EffectType::CornerPin {
        top_left,
        top_right,
        bottom_right,
        bottom_left,
    };

    // Update existing CornerPin effect or push a new one
    if let Some(existing) = layer.effects.iter_mut().find(|e| {
        matches!(
            e.effect_type,
            crate::core::timeline::EffectType::CornerPin { .. }
        )
    }) {
        existing.effect_type = corner_pin_type;
    } else {
        layer.effects.push(crate::core::timeline::Effect {
            id: format!("effect_corner_pin_{}", layer.effects.len() + 1),
            name: "Corner Pin (Perspective Track)".to_string(),
            effect_type: corner_pin_type,
            enabled: true,
        });
    }

    true
}
