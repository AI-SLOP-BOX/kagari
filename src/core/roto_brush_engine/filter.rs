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

pub(crate) fn color_prototypes(samples: &[[f32; 3]], max_prototypes: usize) -> Vec<[f32; 3]> {
    if samples.is_empty() || max_prototypes == 0 {
        return Vec::new();
    }
    let mut centers = vec![samples[0]];
    while centers.len() < max_prototypes.min(samples.len()) {
        let Some((farthest, distance)) = samples
            .iter()
            .copied()
            .map(|sample| {
                let nearest = centers
                    .iter()
                    .map(|center| color_dist_sq(sample, *center))
                    .fold(f32::INFINITY, f32::min);
                (sample, nearest)
            })
            .max_by(|left, right| left.1.total_cmp(&right.1))
        else {
            break;
        };
        if distance <= f32::EPSILON {
            break;
        }
        centers.push(farthest);
    }

    // A few bounded k-means iterations turn the farthest-point seeds into
    // representative colors while retaining distinct appearance clusters.
    for _ in 0..6 {
        let mut sums = vec![[0.0f32; 3]; centers.len()];
        let mut counts = vec![0usize; centers.len()];
        for sample in samples {
            let nearest = centers
                .iter()
                .enumerate()
                .min_by(|(_, left), (_, right)| {
                    color_dist_sq(*sample, **left).total_cmp(&color_dist_sq(*sample, **right))
                })
                .map(|(index, _)| index)
                .unwrap_or(0);
            for (channel, value) in sample.iter().enumerate() {
                sums[nearest][channel] += value;
            }
            counts[nearest] += 1;
        }
        for index in 0..centers.len() {
            if counts[index] > 0 {
                let count = counts[index] as f32;
                centers[index] = [
                    sums[index][0] / count,
                    sums[index][1] / count,
                    sums[index][2] / count,
                ];
            }
        }
    }
    centers
}

pub(crate) fn distance_field_from_points(width: u32, height: u32, points: &[[f32; 2]]) -> Vec<f32> {
    distance_field_from_points_cancellable(width, height, points, || false).unwrap_or_default()
}

pub(crate) fn distance_field_from_points_cancellable(
    width: u32,
    height: u32,
    points: &[[f32; 2]],
    mut is_cancelled: impl FnMut() -> bool,
) -> Option<Vec<f32>> {
    distance_field_from_point_slices_cancellable(width, height, &[points], &mut is_cancelled)
}

pub(crate) fn distance_field_from_point_slices_cancellable(
    width: u32,
    height: u32,
    point_sets: &[&[[f32; 2]]],
    is_cancelled: &mut impl FnMut() -> bool,
) -> Option<Vec<f32>> {
    if is_cancelled() {
        return None;
    }
    let Some(len) = (width as usize).checked_mul(height as usize) else {
        return Some(Vec::new());
    };
    if width == 0
        || height == 0
        || width > i32::MAX as u32
        || height > i32::MAX as u32
        || len > super::matte::MAX_ROTO_FRAME_PIXELS
    {
        return Some(Vec::new());
    }
    let mut distance = Vec::new();
    distance.try_reserve_exact(len).ok()?;
    distance.resize(len, f32::INFINITY);
    if point_sets.iter().all(|points| points.is_empty()) {
        return Some(distance);
    }

    let w = width as usize;
    let h = height as usize;
    let mut seeds = Vec::new();
    seeds.try_reserve_exact(len).ok()?;
    seeds.resize(len, false);
    for points in point_sets {
        for point in *points {
            if is_cancelled() {
                return None;
            }
            if !point[0].is_finite() || !point[1].is_finite() {
                continue;
            }
            let x = (point[0].round() as i64).clamp(0, width as i64 - 1) as usize;
            let y = (point[1].round() as i64).clamp(0, height as i64 - 1) as usize;
            seeds[y * w + x] = true;
        }
    }

    let mut horizontal = Vec::new();
    horizontal.try_reserve_exact(len).ok()?;
    horizontal.resize(len, f32::INFINITY);
    let max_dimension = w.max(h);
    let mut line_input = Vec::new();
    line_input.try_reserve_exact(max_dimension).ok()?;
    line_input.resize(max_dimension, f32::INFINITY);
    let mut line_output = Vec::new();
    line_output.try_reserve_exact(max_dimension).ok()?;
    line_output.resize(max_dimension, f32::INFINITY);
    let mut sites = Vec::new();
    sites.try_reserve_exact(max_dimension).ok()?;
    sites.resize(max_dimension, 0usize);
    let mut boundaries = Vec::new();
    boundaries
        .try_reserve_exact(max_dimension.checked_add(1)?)
        .ok()?;
    boundaries.resize(max_dimension + 1, 0.0f64);
    for y in 0..h {
        if is_cancelled() {
            return None;
        }
        let start = y * w;
        let input = &mut line_input[..w];
        for (value, &seed) in input.iter_mut().zip(&seeds[start..start + w]) {
            *value = if seed { 0.0 } else { f32::INFINITY };
        }
        squared_distance_transform_1d_into(
            input,
            &mut line_output[..w],
            &mut sites[..w],
            &mut boundaries[..=w],
        );
        horizontal[start..start + w].copy_from_slice(&line_output[..w]);
    }
    for x in 0..w {
        if is_cancelled() {
            return None;
        }
        for y in 0..h {
            line_input[y] = horizontal[y * w + x];
        }
        squared_distance_transform_1d_into(
            &line_input[..h],
            &mut line_output[..h],
            &mut sites[..h],
            &mut boundaries[..=h],
        );
        for y in 0..h {
            distance[y * w + x] = line_output[y];
        }
    }
    Some(distance)
}

pub(crate) fn geodesic_distance_field(
    pixels: &[u8],
    width: u32,
    height: u32,
    points: &[[f32; 2]],
    mut is_cancelled: impl FnMut() -> bool,
) -> Option<Vec<u32>> {
    let len = (width as usize).checked_mul(height as usize)?;
    let rgba_len = len.checked_mul(4)?;
    if width == 0 || height == 0 || pixels.len() != rgba_len {
        return Some(Vec::new());
    }
    if len >= u32::MAX as usize {
        return None;
    }

    let mut distances = Vec::new();
    distances.try_reserve_exact(len).ok()?;
    distances.resize(len, u32::MAX);
    if points.is_empty() {
        return Some(distances);
    }
    let mut pending = IndexedMinHeap::try_new(len)?;
    for point in points {
        if is_cancelled() {
            return None;
        }
        if !point[0].is_finite() || !point[1].is_finite() {
            continue;
        }
        let x = (point[0].round() as i64).clamp(0, width as i64 - 1) as usize;
        let y = (point[1].round() as i64).clamp(0, height as i64 - 1) as usize;
        let index = y * width as usize + x;
        if distances[index] != 0 {
            distances[index] = 0;
            pending.insert_or_decrease(index as u32, &distances);
        }
    }

    let width = width as usize;
    let height = height as usize;
    let neighbors = [
        (-1isize, -1isize, 14u32),
        (0, -1, 10),
        (1, -1, 14),
        (-1, 0, 10),
        (1, 0, 10),
        (-1, 1, 14),
        (0, 1, 10),
        (1, 1, 14),
    ];
    let mut visits = 0usize;
    while let Some(index) = pending.pop_min(&distances) {
        visits += 1;
        if visits & 0x0fff == 0 && is_cancelled() {
            return None;
        }

        let index = index as usize;
        let distance = distances[index];
        let x = (index % width) as isize;
        let y = (index / width) as isize;
        let current_pixel = index * 4;
        for (dx, dy, step_cost) in neighbors {
            let nx = x + dx;
            let ny = y + dy;
            if nx < 0 || ny < 0 || nx >= width as isize || ny >= height as isize {
                continue;
            }
            let next = ny as usize * width + nx as usize;
            let next_pixel = next * 4;
            let color_distance_sq = (0..3)
                .map(|channel| {
                    let delta = pixels[current_pixel + channel] as i32
                        - pixels[next_pixel + channel] as i32;
                    (delta * delta) as u32
                })
                .sum::<u32>();
            let edge_cost = step_cost + color_distance_sq / 256;
            let candidate = distance.saturating_add(edge_cost);
            if candidate < distances[next] {
                distances[next] = candidate;
                pending.insert_or_decrease(next as u32, &distances);
            }
        }
    }
    Some(distances)
}

struct IndexedMinHeap {
    indices: Vec<u32>,
    positions: Vec<u32>,
}

impl IndexedMinHeap {
    const NOT_QUEUED: u32 = u32::MAX;

    fn try_new(capacity: usize) -> Option<Self> {
        let mut indices = Vec::new();
        indices.try_reserve_exact(capacity).ok()?;
        let mut positions = Vec::new();
        positions.try_reserve_exact(capacity).ok()?;
        positions.resize(capacity, Self::NOT_QUEUED);
        Some(Self { indices, positions })
    }

    fn insert_or_decrease(&mut self, index: u32, distances: &[u32]) {
        let position = if self.positions[index as usize] == Self::NOT_QUEUED {
            let position = self.indices.len();
            self.indices.push(index);
            self.positions[index as usize] = position as u32;
            position
        } else {
            self.positions[index as usize] as usize
        };
        self.sift_up(position, distances);
    }

    fn pop_min(&mut self, distances: &[u32]) -> Option<u32> {
        let minimum = *self.indices.first()?;
        let last = self.indices.pop().expect("heap root exists");
        self.positions[minimum as usize] = Self::NOT_QUEUED;
        if !self.indices.is_empty() {
            self.indices[0] = last;
            self.positions[last as usize] = 0;
            self.sift_down(0, distances);
        }
        Some(minimum)
    }

    fn sift_up(&mut self, mut position: usize, distances: &[u32]) {
        while position > 0 {
            let parent = (position - 1) / 2;
            if !self.less(position, parent, distances) {
                break;
            }
            self.swap(position, parent);
            position = parent;
        }
    }

    fn sift_down(&mut self, mut position: usize, distances: &[u32]) {
        loop {
            let left = position * 2 + 1;
            if left >= self.indices.len() {
                break;
            }
            let right = left + 1;
            let child = if right < self.indices.len() && self.less(right, left, distances) {
                right
            } else {
                left
            };
            if !self.less(child, position, distances) {
                break;
            }
            self.swap(position, child);
            position = child;
        }
    }

    fn less(&self, left: usize, right: usize, distances: &[u32]) -> bool {
        let left_index = self.indices[left];
        let right_index = self.indices[right];
        distances[left_index as usize]
            .cmp(&distances[right_index as usize])
            .then_with(|| left_index.cmp(&right_index))
            .is_lt()
    }

    fn swap(&mut self, left: usize, right: usize) {
        self.indices.swap(left, right);
        self.positions[self.indices[left] as usize] = left as u32;
        self.positions[self.indices[right] as usize] = right as u32;
    }
}

pub(crate) fn squared_distance_transform_1d_into(
    input: &[f32],
    output: &mut [f32],
    sites: &mut [usize],
    boundaries: &mut [f64],
) {
    debug_assert_eq!(input.len(), output.len());
    debug_assert!(sites.len() >= input.len());
    debug_assert!(boundaries.len() > input.len());
    output.fill(f32::INFINITY);
    if input.is_empty() {
        return;
    }
    let Some(first) = input.iter().position(|value| value.is_finite()) else {
        return;
    };

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
}

pub(crate) fn color_dist_sq(c1: [f32; 3], c2: [f32; 3]) -> f32 {
    let dr = c1[0] - c2[0];
    let dg = c1[1] - c2[1];
    let db = c1[2] - c2[2];
    dr * dr + dg * dg + db * db
}

pub(crate) fn refine_edge_matte(matte: &mut [u8], width: u32, height: u32, feather: f32) {
    let _ = refine_edge_matte_cancellable(matte, width, height, feather, || false);
}

pub(crate) fn refine_edge_matte_cancellable(
    matte: &mut [u8],
    width: u32,
    height: u32,
    feather: f32,
    mut is_cancelled: impl FnMut() -> bool,
) -> Option<()> {
    let Some(len) = (width as usize).checked_mul(height as usize) else {
        return Some(());
    };
    if width == 0 || height == 0 || matte.len() != len {
        return Some(());
    }
    let feather = if feather.is_finite() {
        feather.clamp(0.0, 64.0)
    } else {
        0.0
    };
    let radius = feather.round() as usize;
    if radius == 0 {
        return Some(());
    }
    if is_cancelled() {
        return None;
    }

    let w = width as usize;
    let h = height as usize;
    let kernel_size = (radius * 2 + 1) as f32;
    let mut source = Vec::new();
    source.try_reserve_exact(len).ok()?;
    source.extend(matte.iter().map(|&value| value as f32));
    let mut horizontal = Vec::new();
    horizontal.try_reserve_exact(len).ok()?;
    horizontal.resize(len, 0.0f32);

    for y in 0..h {
        if is_cancelled() {
            return None;
        }
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
        if is_cancelled() {
            return None;
        }
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
    Some(())
}

#[cfg(test)]
mod tests {
    use super::IndexedMinHeap;

    #[test]
    fn indexed_geodesic_frontier_decreases_keys_without_duplicate_entries() {
        let mut distances = [50, 30, 20, 40];
        let mut frontier = IndexedMinHeap::try_new(distances.len())
            .expect("frontier should allocate for a small image");
        frontier.insert_or_decrease(0, &distances);
        frontier.insert_or_decrease(1, &distances);
        frontier.insert_or_decrease(2, &distances);

        distances[0] = 10;
        frontier.insert_or_decrease(0, &distances);

        assert_eq!(frontier.indices.len(), 3);
        assert_eq!(frontier.pop_min(&distances), Some(0));
        assert_eq!(frontier.pop_min(&distances), Some(2));
        assert_eq!(frontier.pop_min(&distances), Some(1));
        assert_eq!(frontier.pop_min(&distances), None);
    }
}
