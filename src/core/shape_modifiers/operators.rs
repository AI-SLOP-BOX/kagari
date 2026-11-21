use crate::core::mask::MaskVertex;

/// Pucker & Bloat options matching After Effects Shape Pucker & Bloat modifier.
#[derive(Debug, Clone)]
pub struct PuckerBloatOptions {
    pub amount: f32, // Percentage (-100.0 = full Pucker, +100.0 = full Bloat)
}

/// Zig Zag options matching After Effects Shape Zig Zag modifier.
#[derive(Debug, Clone)]
pub struct ZigZagOptions {
    pub size: f32,               // Ridge amplitude in pixels
    pub ridges_per_segment: u32, // Number of ridges per path segment
    pub smooth: bool,            // Smooth curve vs sharp corner points
}

/// Applies Pucker & Bloat distortion to a vector shape path.
pub fn apply_pucker_bloat(
    vertices: &[MaskVertex],
    options: &PuckerBloatOptions,
) -> Vec<MaskVertex> {
    if vertices.is_empty() || options.amount.abs() < 0.001 {
        return vertices.to_vec();
    }

    // Compute centroid of shape vertices
    let mut center = [0.0f32, 0.0f32];
    for v in vertices {
        center[0] += v.position[0];
        center[1] += v.position[1];
    }
    let n = vertices.len() as f32;
    center[0] /= n;
    center[1] /= n;

    let factor = options.amount * 0.01;
    let mut modified = Vec::with_capacity(vertices.len());

    for v in vertices {
        let mut v_new = v.clone();

        // Vector from center to vertex position
        let vx = v.position[0] - center[0];
        let vy = v.position[1] - center[1];

        // Shift vertex towards or away from center
        v_new.position[0] += vx * factor;
        v_new.position[1] += vy * factor;

        // Scale Bezier tangents inversely for Pucker / Bloat organic curve bulging
        v_new.tangent_in[0] *= 1.0 - factor;
        v_new.tangent_in[1] *= 1.0 - factor;
        v_new.tangent_out[0] *= 1.0 - factor;
        v_new.tangent_out[1] *= 1.0 - factor;

        modified.push(v_new);
    }

    modified
}

/// Applies Zig Zag (sawtooth / wave) geometric sub-division distortion to a Bezier path.
pub fn apply_zig_zag(vertices: &[MaskVertex], options: &ZigZagOptions) -> Vec<MaskVertex> {
    if vertices.len() < 2 || options.ridges_per_segment == 0 || options.size <= 0.001 {
        return vertices.to_vec();
    }

    let mut result = Vec::new();
    let ridges = options.ridges_per_segment as usize;

    for i in 0..vertices.len() - 1 {
        let v0 = &vertices[i];
        let v1 = &vertices[i + 1];

        result.push(v0.clone());

        let dx = v1.position[0] - v0.position[0];
        let dy = v1.position[1] - v0.position[1];
        let len = (dx * dx + dy * dy).sqrt().max(0.001);

        let normal = [-dy / len, dx / len];

        for r in 1..ridges {
            let t = r as f32 / ridges as f32;
            let px = v0.position[0] + dx * t;
            let py = v0.position[1] + dy * t;

            let side = if r % 2 == 1 { 1.0 } else { -1.0 };
            let offset_x = normal[0] * options.size * side;
            let offset_y = normal[1] * options.size * side;

            let mut ridge_vertex = MaskVertex::new(px + offset_x, py + offset_y);
            if !options.smooth {
                ridge_vertex.tangent_in = [0.0, 0.0];
                ridge_vertex.tangent_out = [0.0, 0.0];
            }
            result.push(ridge_vertex);
        }
    }

    if let Some(last) = vertices.last() {
        result.push(last.clone());
    }

    result
}

/// Applies Round Corners modifier to a Bezier vertex path.
pub fn apply_round_corners(vertices: &[MaskVertex], radius: f32, closed: bool) -> Vec<MaskVertex> {
    if vertices.len() < 3 || radius <= 0.001 {
        return vertices.to_vec();
    }

    let mut result = Vec::new();
    let n = vertices.len();

    for i in 0..n {
        if !closed && (i == 0 || i == n - 1) {
            result.push(vertices[i].clone());
            continue;
        }

        let prev = if i == 0 {
            &vertices[n - 1]
        } else {
            &vertices[i - 1]
        };
        let curr = &vertices[i];
        let next = if i == n - 1 {
            &vertices[0]
        } else {
            &vertices[i + 1]
        };

        // Vectors from curr to prev and curr to next
        let v_prev = [
            prev.position[0] - curr.position[0],
            prev.position[1] - curr.position[1],
        ];
        let v_next = [
            next.position[0] - curr.position[0],
            next.position[1] - curr.position[1],
        ];

        let len_prev = (v_prev[0].powi(2) + v_prev[1].powi(2)).sqrt().max(0.001);
        let len_next = (v_next[0].powi(2) + v_next[1].powi(2)).sqrt().max(0.001);

        let d_prev = radius.min(len_prev * 0.45);
        let d_next = radius.min(len_next * 0.45);

        let p_in = [
            curr.position[0] + (v_prev[0] / len_prev) * d_prev,
            curr.position[1] + (v_prev[1] / len_prev) * d_prev,
        ];
        let p_out = [
            curr.position[0] + (v_next[0] / len_next) * d_next,
            curr.position[1] + (v_next[1] / len_next) * d_next,
        ];

        let mut v_in = MaskVertex::new(p_in[0], p_in[1]);
        let mut v_out = MaskVertex::new(p_out[0], p_out[1]);

        let kappa = 0.552_284_8_f32; // Standard Bezier circular approximation constant
        v_in.tangent_out = [
            (curr.position[0] - p_in[0]) * kappa,
            (curr.position[1] - p_in[1]) * kappa,
        ];
        v_out.tangent_in = [
            (curr.position[0] - p_out[0]) * kappa,
            (curr.position[1] - p_out[1]) * kappa,
        ];

        result.push(v_in);
        result.push(v_out);
    }

    result
}

/// Trims a polyline or closed Bezier path between start (0..100) and end (0..100) with offset.
pub fn trim_path_vertices(
    vertices: &[MaskVertex],
    start_pct: f32,
    end_pct: f32,
    offset_pct: f32,
    closed: bool,
) -> Vec<MaskVertex> {
    if vertices.len() < 2 {
        return vertices.to_vec();
    }

    let start = (start_pct * 0.01 + offset_pct * 0.01).rem_euclid(1.0);
    let end = (end_pct * 0.01 + offset_pct * 0.01).rem_euclid(1.0);

    if (start - end).abs() < 1e-4 {
        return Vec::new();
    }

    // Measure total cumulative segment lengths
    let n = vertices.len();
    let num_segs = if closed { n } else { n - 1 };
    let mut cum_lens = vec![0.0f32; num_segs + 1];
    let mut total_len = 0.0f32;

    for i in 0..num_segs {
        let p0 = vertices[i].position;
        let p1 = vertices[(i + 1) % n].position;
        let seg_len = ((p1[0] - p0[0]).powi(2) + (p1[1] - p0[1]).powi(2)).sqrt();
        total_len += seg_len;
        cum_lens[i + 1] = total_len;
    }

    if total_len < 1e-4 {
        return vertices.to_vec();
    }

    let sample_at = |t: f32| -> MaskVertex {
        let target_dist = t.clamp(0.0, 1.0) * total_len;
        for i in 0..num_segs {
            if target_dist <= cum_lens[i + 1] || i == num_segs - 1 {
                let seg_l = (cum_lens[i + 1] - cum_lens[i]).max(1e-5);
                let frac = ((target_dist - cum_lens[i]) / seg_l).clamp(0.0, 1.0);
                let p0 = &vertices[i];
                let p1 = &vertices[(i + 1) % n];
                let px = p0.position[0] + (p1.position[0] - p0.position[0]) * frac;
                let py = p0.position[1] + (p1.position[1] - p0.position[1]) * frac;
                return MaskVertex::new(px, py);
            }
        }
        vertices[0].clone()
    };

    let mut result = Vec::new();
    if start < end {
        result.push(sample_at(start));
        for i in 0..num_segs {
            let seg_t = cum_lens[i + 1] / total_len;
            if seg_t > start && seg_t < end {
                result.push(vertices[(i + 1) % n].clone());
            }
        }
        result.push(sample_at(end));
    } else {
        // Wrapped around start > end
        result.push(sample_at(start));
        for i in 0..num_segs {
            let seg_t = cum_lens[i + 1] / total_len;
            if seg_t > start {
                result.push(vertices[(i + 1) % n].clone());
            }
        }
        for i in 0..num_segs {
            let seg_t = cum_lens[i + 1] / total_len;
            if seg_t < end {
                result.push(vertices[(i + 1) % n].clone());
            }
        }
        result.push(sample_at(end));
    }

    result
}

/// Line join style for Offset Paths modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OffsetPathsJoin {
    #[default]
    Miter,
    Round,
    Bevel,
}

/// Applies Offset Paths modifier to a polygon path.
pub fn apply_offset_paths(
    vertices: &[MaskVertex],
    amount: f32,
    join: OffsetPathsJoin,
    miter_limit: f32,
) -> Vec<MaskVertex> {
    let n = vertices.len();
    if n < 3 || amount.abs() < 1e-4 {
        return vertices.to_vec();
    }

    // Compute signed polygon area to detect winding direction
    let mut area = 0.0f32;
    for i in 0..n {
        let j = (i + 1) % n;
        area += vertices[i].position[0] * vertices[j].position[1]
            - vertices[j].position[0] * vertices[i].position[1];
    }
    let sign = if area > 0.0 { -1.0 } else { 1.0 };
    let eff_amount = amount * sign;

    let mut result = Vec::with_capacity(n * 2);

    for i in 0..n {
        let prev = &vertices[(i + n - 1) % n];
        let curr = &vertices[i];
        let next = &vertices[(i + 1) % n];

        let d1 = [
            curr.position[0] - prev.position[0],
            curr.position[1] - prev.position[1],
        ];
        let d2 = [
            next.position[0] - curr.position[0],
            next.position[1] - curr.position[1],
        ];

        let len1 = (d1[0] * d1[0] + d1[1] * d1[1]).sqrt().max(1e-4);
        let len2 = (d2[0] * d2[0] + d2[1] * d2[1]).sqrt().max(1e-4);

        // Outward normals
        let n1 = [-d1[1] / len1, d1[0] / len1];
        let n2 = [-d2[1] / len2, d2[0] / len2];

        let avg_n = [n1[0] + n2[0], n1[1] + n2[1]];
        let avg_len = (avg_n[0] * avg_n[0] + avg_n[1] * avg_n[1]).sqrt().max(1e-4);
        let bisector = [avg_n[0] / avg_len, avg_n[1] / avg_len];

        let cos_half = n1[0] * bisector[0] + n1[1] * bisector[1];
        let miter_len = if cos_half.abs() > 1e-3 {
            (eff_amount / cos_half).clamp(
                -miter_limit * eff_amount.abs(),
                miter_limit * eff_amount.abs(),
            )
        } else {
            eff_amount
        };

        match join {
            OffsetPathsJoin::Miter => {
                let offset_pos = [
                    curr.position[0] + bisector[0] * miter_len,
                    curr.position[1] + bisector[1] * miter_len,
                ];
                result.push(MaskVertex::new(offset_pos[0], offset_pos[1]));
            }
            OffsetPathsJoin::Bevel | OffsetPathsJoin::Round => {
                let p1 = [
                    curr.position[0] + n1[0] * amount,
                    curr.position[1] + n1[1] * amount,
                ];
                let p2 = [
                    curr.position[0] + n2[0] * amount,
                    curr.position[1] + n2[1] * amount,
                ];
                result.push(MaskVertex::new(p1[0], p1[1]));
                result.push(MaskVertex::new(p2[0], p2[1]));
            }
        }
    }

    result
}
