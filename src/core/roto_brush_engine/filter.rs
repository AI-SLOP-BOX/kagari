//! Euclidean distance field and edge refinement filters for alpha mattes.

pub(crate) fn compute_mean_color(samples: &[[f32; 3]]) -> Option<[f32; 3]> {
    if samples.is_empty() {
        return None;
    }
    let mut sum = [0.0f32; 3];
    for s in samples {
        sum[0] += s[0];
        sum[1] += s[1];
        sum[2] += s[2];
    }
    let len = samples.len() as f32;
    Some([sum[0] / len, sum[1] / len, sum[2] / len])
}

pub(crate) fn distance_field_from_points(width: u32, height: u32, points: &[[f32; 2]]) -> Vec<f32> {
    let Some(len) = (width as usize).checked_mul(height as usize) else {
        return Vec::new();
    };
    let mut distance = vec![f32::INFINITY; len];
    if width == 0 || height == 0 || points.is_empty() {
        return distance;
    }

    let w = width as usize;
    let h = height as usize;
    let mut seeds = vec![false; len];
    for point in points {
        if !point[0].is_finite() || !point[1].is_finite() {
            continue;
        }
        let x = (point[0].round() as i64).clamp(0, width as i64 - 1) as usize;
        let y = (point[1].round() as i64).clamp(0, height as i64 - 1) as usize;
        seeds[y * w + x] = true;
    }

    let mut horizontal = vec![f32::INFINITY; len];
    for y in 0..h {
        let start = y * w;
        let row: Vec<f32> = seeds[start..start + w]
            .iter()
            .map(|&seed| if seed { 0.0 } else { f32::INFINITY })
            .collect();
        horizontal[start..start + w].copy_from_slice(&squared_distance_transform_1d(&row));
    }
    for x in 0..w {
        let column: Vec<f32> = (0..h).map(|y| horizontal[y * w + x]).collect();
        let transformed = squared_distance_transform_1d(&column);
        for (y, value) in transformed.into_iter().enumerate() {
            distance[y * w + x] = value;
        }
    }
    distance
}

pub(crate) fn squared_distance_transform_1d(input: &[f32]) -> Vec<f32> {
    let mut output = vec![f32::INFINITY; input.len()];
    let Some(first) = input.iter().position(|value| value.is_finite()) else {
        return output;
    };

    let mut sites = vec![0usize; input.len()];
    let mut boundaries = vec![0.0f64; input.len() + 1];
    let mut envelope = 0usize;
    sites[0] = first;
    boundaries[0] = f64::NEG_INFINITY;
    boundaries[1] = f64::INFINITY;

    for q in first + 1..input.len() {
        if !input[q].is_finite() {
            continue;
        }
        let qf = q as f64;
        loop {
            let p = sites[envelope];
            let pf = p as f64;
            let intersection =
                ((input[q] as f64 + qf * qf) - (input[p] as f64 + pf * pf)) / (2.0 * (qf - pf));
            if intersection > boundaries[envelope] {
                envelope += 1;
                sites[envelope] = q;
                boundaries[envelope] = intersection;
                boundaries[envelope + 1] = f64::INFINITY;
                break;
            }
            envelope -= 1;
        }
    }

    envelope = 0;
    for (q, out) in output.iter_mut().enumerate() {
        while boundaries[envelope + 1] < q as f64 {
            envelope += 1;
        }
        let delta = q as f64 - sites[envelope] as f64;
        *out = (delta * delta + input[sites[envelope]] as f64) as f32;
    }
    output
}

pub(crate) fn color_dist_sq(c1: [f32; 3], c2: [f32; 3]) -> f32 {
    let dr = c1[0] - c2[0];
    let dg = c1[1] - c2[1];
    let db = c1[2] - c2[2];
    dr * dr + dg * dg + db * db
}

pub(crate) fn refine_edge_matte(matte: &mut [u8], width: u32, height: u32, feather: f32) {
    let Some(len) = (width as usize).checked_mul(height as usize) else {
        return;
    };
    if width == 0 || height == 0 || matte.len() != len {
        return;
    }
    let feather = if feather.is_finite() {
        feather.clamp(0.0, 64.0)
    } else {
        0.0
    };
    let radius = feather.round() as usize;
    if radius == 0 {
        return;
    }

    let w = width as usize;
    let h = height as usize;
    let kernel_size = (radius * 2 + 1) as f32;
    let source: Vec<f32> = matte.iter().map(|&value| value as f32).collect();
    let mut horizontal = vec![0.0f32; len];

    for y in 0..h {
        let row = y * w;
        let mut sum = 0.0f32;
        for offset in -(radius as isize)..=radius as isize {
            let x = offset.clamp(0, w as isize - 1) as usize;
            sum += source[row + x];
        }
        for x in 0..w {
            if x > 0 {
                let outgoing = (x as isize - radius as isize - 1).clamp(0, w as isize - 1) as usize;
                let incoming = (x + radius).min(w - 1);
                sum += source[row + incoming] - source[row + outgoing];
            }
            horizontal[row + x] = sum / kernel_size;
        }
    }

    for x in 0..w {
        let mut sum = 0.0f32;
        for offset in -(radius as isize)..=radius as isize {
            let y = offset.clamp(0, h as isize - 1) as usize;
            sum += horizontal[y * w + x];
        }
        for y in 0..h {
            if y > 0 {
                let outgoing = (y as isize - radius as isize - 1).clamp(0, h as isize - 1) as usize;
                let incoming = (y + radius).min(h - 1);
                sum += horizontal[incoming * w + x] - horizontal[outgoing * w + x];
            }
            matte[y * w + x] = (sum / kernel_size).round().clamp(0.0, 255.0) as u8;
        }
    }
}
