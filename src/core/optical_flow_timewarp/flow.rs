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
        Self {
            width,
            height,
            vectors: vec![[0.0, 0.0]; size],
        }
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
    let mut flow = DenseFlowField::new(width, height);
    let Some(size) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|v| v.checked_mul(4))
    else {
        return flow;
    };
    if width == 0 || height == 0 || src_rgba.len() != size || tgt_rgba.len() != size {
        return flow;
    }
    if flow.vectors.len() != (width as usize).saturating_mul(height as usize) {
        return flow;
    }
    let w = width as i32;
    let h = height as i32;
    let block_radius = block_radius.clamp(0, 64);
    let search_radius = search_radius.clamp(0, 128);

    let get_luma = |buf: &[u8], x: i32, y: i32| -> f32 {
        let cx = x.clamp(0, w - 1) as usize;
        let cy = y.clamp(0, h - 1) as usize;
        let idx = (cy * width as usize + cx) * 4;
        0.299 * buf[idx] as f32 + 0.587 * buf[idx + 1] as f32 + 0.114 * buf[idx + 2] as f32
    };

    // Subsampled dense matching for high performance
    let step = 4;
    for by in (0..h).step_by(step as usize) {
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

                    if sad < best_sad {
                        best_sad = sad;
                        best_dx = sdx as f32;
                        best_dy = sdy as f32;
                    }
                }
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

    flow
}
