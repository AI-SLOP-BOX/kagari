pub const MAX_DENSE_FLOW_PIXELS: usize = 16_777_216;

#[derive(Debug, Clone)]
pub struct DenseFlowField {
    pub width: u32,
    pub height: u32,
    pub vectors: Vec<[f32; 2]>, // [dx, dy] per pixel
}

impl DenseFlowField {
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width as usize)
            .checked_mul(height as usize)
            .filter(|&count| count <= MAX_DENSE_FLOW_PIXELS)
            .unwrap_or(0);
        let mut vectors = Vec::new();
        if vectors.try_reserve_exact(size).is_ok() {
            vectors.resize(size, [0.0, 0.0]);
        }
        Self {
            width,
            height,
            vectors,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.width > 0
            && self.height > 0
            && (self.width as usize)
                .checked_mul(self.height as usize)
                .is_some_and(|pixels| {
                    pixels <= MAX_DENSE_FLOW_PIXELS && self.vectors.len() == pixels
                })
    }

    #[inline]
    pub fn get(&self, x: u32, y: u32) -> [f32; 2] {
        if x < self.width && y < self.height {
            self.vectors
                .get(y as usize * self.width as usize + x as usize)
                .copied()
                .unwrap_or([0.0, 0.0])
        } else {
            [0.0, 0.0]
        }
    }
}

/// Computes block-matching dense optical flow vectors from source to target frame.
pub fn compute_dense_optical_flow(
    src_rgba: &[u8],
    tgt_rgba: &[u8],
    width: u32,
    height: u32,
    block_radius: i32,
    search_radius: i32,
) -> DenseFlowField {
    compute_dense_optical_flow_cancellable(
        src_rgba,
        tgt_rgba,
        width,
        height,
        block_radius,
        search_radius,
        &mut || false,
    )
    .unwrap_or(DenseFlowField {
        width,
        height,
        vectors: Vec::new(),
    })
}

/// Computes dense block-matching flow and stops between block rows when
/// `is_cancelled` returns true. `None` indicates cancellation.
pub fn compute_dense_optical_flow_cancellable(
    src_rgba: &[u8],
    tgt_rgba: &[u8],
    width: u32,
    height: u32,
    block_radius: i32,
    search_radius: i32,
    is_cancelled: &mut impl FnMut() -> bool,
) -> Option<DenseFlowField> {
    let mut flow = DenseFlowField::new(width, height);
    let Some(size) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|v| v.checked_mul(4))
    else {
        return Some(flow);
    };
    if width == 0 || height == 0 || src_rgba.len() != size || tgt_rgba.len() != size {
        return Some(flow);
    }
    if !flow.is_complete() {
        return Some(flow);
    }
    let w = width as i32;
    let h = height as i32;
    let block_radius = block_radius.clamp(0, 64);
    let search_radius = search_radius.clamp(0, 128);

    #[cfg(feature = "wgpu")]
    if crate::core::compute_pipeline::gpu_effects_enabled()
        && (1..=4).contains(&block_radius)
        && (1..=16).contains(&search_radius)
        && !is_cancelled()
    {
        if let Some((gpu_field, _conf)) = crate::core::optical_flow_gpu::compute_dense_optical_flow_gpu(
            src_rgba,
            tgt_rgba,
            width,
            height,
            block_radius,
            search_radius,
        ) {
            return Some(gpu_field);
        }
    }

    let get_luma = |buf: &[u8], x: i32, y: i32| -> f32 {
        let cx = x.clamp(0, w - 1) as usize;
        let cy = y.clamp(0, h - 1) as usize;
        let idx = (cy * width as usize + cx) * 4;
        0.299 * buf[idx] as f32 + 0.587 * buf[idx + 1] as f32 + 0.114 * buf[idx + 2] as f32
    };

    // Subsampled dense matching for high performance
    let step = 4;
    for by in (0..h).step_by(step as usize) {
        if is_cancelled() {
            return None;
        }
        for bx in (0..w).step_by(step as usize) {
            let mut best_sad = f32::INFINITY;
            let mut best_dx = 0.0f32;
            let mut best_dy = 0.0f32;

            for sdy in -search_radius..=search_radius {
                for sdx in -search_radius..=search_radius {
                    let mut sad = 0.0f32;
                    for py in -block_radius..=block_radius {
                        for px in -block_radius..=block_radius {
                            let src_val = get_luma(src_rgba, bx + px, by + py);
                            let tgt_val = get_luma(tgt_rgba, bx + px + sdx, by + py + sdy);
                            sad += (src_val - tgt_val).abs();
                        }
                    }

                    let candidate_motion_sq = (sdx * sdx + sdy * sdy) as f32;
                    let best_motion_sq = best_dx * best_dx + best_dy * best_dy;
                    if sad < best_sad || (sad == best_sad && candidate_motion_sq < best_motion_sq) {
                        best_sad = sad;
                        best_dx = sdx as f32;
                        best_dy = sdy as f32;
                    }
                }
            }

            let integer_dx = best_dx as i32;
            let integer_dy = best_dy as i32;
            let block_cost = |shift_x: i32, shift_y: i32| {
                let mut sad = 0.0f32;
                for py in -block_radius..=block_radius {
                    for px in -block_radius..=block_radius {
                        let source = get_luma(src_rgba, bx + px, by + py);
                        let target = get_luma(tgt_rgba, bx + px + shift_x, by + py + shift_y);
                        sad += (source - target).abs();
                    }
                }
                sad
            };
            if integer_dx > -search_radius && integer_dx < search_radius {
                best_dx += parabolic_minimum_offset(
                    block_cost(integer_dx - 1, integer_dy),
                    best_sad,
                    block_cost(integer_dx + 1, integer_dy),
                );
            }
            if integer_dy > -search_radius && integer_dy < search_radius {
                best_dy += parabolic_minimum_offset(
                    block_cost(integer_dx, integer_dy - 1),
                    best_sad,
                    block_cost(integer_dx, integer_dy + 1),
                );
            }

            // Populate block cells
            for dy in 0..step {
                let y = by + dy;
                if y >= h {
                    continue;
                }
                for dx in 0..step {
                    let x = bx + dx;
                    if x >= w {
                        continue;
                    }
                    flow.vectors[(y as u32 * width + x as u32) as usize] = [best_dx, best_dy];
                }
            }
        }
    }

    Some(flow)
}

fn parabolic_minimum_offset(before: f32, center: f32, after: f32) -> f32 {
    let curvature = before - 2.0 * center + after;
    if !before.is_finite() || !center.is_finite() || !after.is_finite() || curvature <= 1.0e-6 {
        return 0.0;
    }
    (0.5 * (before - after) / curvature).clamp(-0.5, 0.5)
}
