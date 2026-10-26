use super::*;

/// CPU-rasterized combined mask coverage for one layer, ready for GPU upload.
pub(crate) struct MaskRaster {
    /// Hash of the mask INPUTS (scaled shapes + output size) — identical
    /// inputs across frames hit the cache and share one upload.
    pub key: u64,
    /// RGBA8 pixels; rgb = white, a = combined coverage.
    pub pixels: Vec<u8>,
}

/// Content-addressed FIFO cache so static masks skip the (expensive) EDT
/// re-raster while OTHER layers keep animating during playback.
#[derive(Default)]
pub(super) struct MaskRasterCache {
    order: Vec<u64>,
    map: std::collections::HashMap<u64, std::sync::Arc<MaskRaster>>,
}

impl MaskRasterCache {
    pub(super) const CAP: usize = 6;

    pub(super) fn get(&self, key: u64) -> Option<std::sync::Arc<MaskRaster>> {
        self.map.get(&key).cloned()
    }

    pub(super) fn insert(&mut self, raster: std::sync::Arc<MaskRaster>) {
        let key = raster.key;
        if self.map.insert(key, raster).is_none() {
            self.order.push(key);
            while self.order.len() > Self::CAP {
                let evicted = self.order.remove(0);
                self.map.remove(&evicted);
            }
        }
    }
}

/// Hashes only the mask INPUTS — cheaper than hashing the full pixel buffer
/// and enables lookup before any rasterization work.
fn mask_input_key(shapes: &[MaskShape], width: u32, height: u32) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    width.hash(&mut hasher);
    height.hash(&mut hasher);
    for shape in shapes {
        for pt in &shape.poly {
            pt[0].to_bits().hash(&mut hasher);
            pt[1].to_bits().hash(&mut hasher);
        }
        (shape.mode as u8).hash(&mut hasher);
        shape.opacity.to_bits().hash(&mut hasher);
        shape.inverted.hash(&mut hasher);
        shape.feather.to_bits().hash(&mut hasher);
    }
    hasher.finish()
}

/// Even-odd scanline polygon fill with pixel-center sampling.
/// Returns per-pixel binary coverage in row-major order.
fn rasterize_polygon_evenodd(poly: &[[f32; 2]], width: u32, height: u32) -> Vec<f32> {
    let mut cov = vec![0.0f32; (width as usize) * (height as usize)];
    let n = poly.len();
    if n < 3 || width == 0 || height == 0 {
        return cov;
    }
    for y in 0..height {
        let py = y as f32 + 0.5;
        let mut xs: Vec<f32> = Vec::new();
        let mut j = n - 1;
        for i in 0..n {
            let (x1, y1) = (poly[j][0], poly[j][1]);
            let (x2, y2) = (poly[i][0], poly[i][1]);
            if (y1 <= py && y2 > py) || (y2 <= py && y1 > py) {
                let t = (py - y1) / (y2 - y1);
                xs.push(x1 + t * (x2 - x1));
            }
            j = i;
        }
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let row = y as usize * width as usize;
        let mut k = 0;
        while k + 1 < xs.len() {
            let xa = xs[k];
            let xb = xs[k + 1];
            if xb <= xa {
                k += 2;
                continue;
            }
            let x_start = ((xa - 0.5).ceil().max(0.0)) as usize;
            let x_end_incl = ((xb - 0.5).floor().min(width as f32 - 1.0)).max(-1.0) as usize;
            for x in x_start..=(x_end_incl.min(width as usize - 1)) {
                let cx = x as f32 + 0.5;
                if cx >= xa && cx < xb {
                    cov[row + x] = 1.0;
                }
            }
            k += 2;
        }
    }
    cov
}

/// One pass of the Felzenszwalb–Huttenlocher 1D squared distance transform.
/// `f` holds per-site squared distances (INF where no target site exists).
fn edt_1d(f: &[f64], d: &mut [f64], v: &mut [usize], z: &mut [f64]) {
    let n = f.len();
    let mut k = 0usize;
    v[0] = 0;
    z[0] = f64::NEG_INFINITY;
    z[1] = f64::INFINITY;
    for q in 1..n {
        let mut s = ((f[q] + (q * q) as f64) - (f[v[k]] + (v[k] * v[k]) as f64))
            / (2.0 * (q - v[k]) as f64);
        while s <= z[k] {
            k -= 1;
            s = ((f[q] + (q * q) as f64) - (f[v[k]] + (v[k] * v[k]) as f64))
                / (2.0 * (q - v[k]) as f64);
        }
        k += 1;
        v[k] = q;
        z[k] = s;
        z[k + 1] = f64::INFINITY;
    }
    k = 0;
    for (q, dq) in d.iter_mut().enumerate() {
        while z[k + 1] < q as f64 {
            k += 1;
        }
        // Cast before subtracting: with all-INF rows the envelope can hold
        // sites past q, and usize underflow would panic in debug builds.
        let dx = q as f64 - v[k] as f64;
        *dq = dx * dx + f[v[k]];
    }
}

/// Exact Euclidean distance (px) from every pixel to the nearest pixel where
/// `binary == target`. Two-pass separable EDT — O(w·h), branch-stable, and
/// fully deterministic for identical inputs.
fn edt_2d(binary: &[bool], width: u32, height: u32, target: bool) -> Vec<f32> {
    const INF: f64 = 1.0e18;
    let w = width as usize;
    let h = height as usize;
    if w == 0 || h == 0 {
        return Vec::new();
    }
    let mut grid = vec![0.0f64; w * h];
    for (i, g) in grid.iter_mut().enumerate() {
        *g = if binary[i] == target { 0.0 } else { INF };
    }
    let maxlen = w.max(h);
    let mut f = vec![0.0f64; maxlen];
    let mut d = vec![0.0f64; maxlen];
    let mut v = vec![0usize; maxlen];
    let mut z = vec![0.0f64; maxlen + 1];

    for y in 0..h {
        f[..w].copy_from_slice(&grid[y * w..(y + 1) * w]);
        edt_1d(&f[..w], &mut d[..w], &mut v[..w], &mut z[..w + 1]);
        grid[y * w..(y + 1) * w].copy_from_slice(&d[..w]);
    }
    for x in 0..w {
        for y in 0..h {
            f[y] = grid[y * w + x];
        }
        edt_1d(&f[..h], &mut d[..h], &mut v[..h], &mut z[..h + 1]);
        for y in 0..h {
            grid[y * w + x] = d[y];
        }
    }
    grid.into_iter().map(|g| g.sqrt() as f32).collect()
}

/// Replaces a binary coverage map with an AE-style feathered matte: alpha
/// ramps linearly across `feather` pixels centered on the polygon boundary.
/// Distances are capped so uniform regions clamp cleanly (no INF−INF NaN).
fn feather_coverage(cov: &[f32], width: u32, height: u32, feather: f32) -> Vec<f32> {
    let bin: Vec<bool> = cov.iter().map(|&c| c > 0.5).collect();
    let d_to_uncovered = edt_2d(&bin, width, height, false); // inside → edge
    let d_to_covered = edt_2d(&bin, width, height, true); // outside → edge
    let cap = (width + height) as f32;
    let half = feather.max(0.5);
    cov.iter()
        .zip(d_to_uncovered)
        .zip(d_to_covered)
        .map(|((_, di), do_)| {
            let di = di.min(cap);
            let dout = do_.min(cap);
            let signed = di - dout; // >0 inside the shape
            ((signed / half) * 0.5 + 0.5).clamp(0.0, 1.0)
        })
        .collect()
}

/// Combines evaluated mask shapes into a single coverage buffer using AE's
/// mask-mode semantics. A Subtract FIRST mask starts from a full frame so it
/// carves material away (subtraction against an empty base is a no-op);
/// an inverted Add already carries its complement in its own coverage.
pub(crate) fn combine_mask_shapes(
    shapes: &[MaskShape],
    width: u32,
    height: u32,
) -> Option<Vec<f32>> {
    use crate::core::mask::MaskMode;
    let n = (width as usize) * (height as usize);
    if n == 0 {
        return None;
    }
    let mut acc = vec![0.0f32; n];
    let mut any = false;
    let mut first = true;
    for shape in shapes {
        if shape.mode == MaskMode::None || shape.poly.len() < 3 {
            continue;
        }
        let mut cov = rasterize_polygon_evenodd(&shape.poly, width, height);
        if shape.feather >= 0.5 {
            cov = feather_coverage(&cov, width, height, shape.feather);
        }
        let op = shape.opacity.clamp(0.0, 1.0);
        if op < 1.0 {
            for c in cov.iter_mut() {
                *c *= op;
            }
        }
        if shape.inverted {
            for c in cov.iter_mut() {
                *c = 1.0 - *c;
            }
        }
        if first && shape.mode == MaskMode::Subtract {
            acc.iter_mut().for_each(|a| *a = 1.0);
        }
        for (a, b) in acc.iter_mut().zip(cov.iter()) {
            *a = match shape.mode {
                MaskMode::Add | MaskMode::Lighten => *a + (*b * (1.0 - *a)),
                MaskMode::Subtract => *a * (1.0 - *b),
                MaskMode::Intersect | MaskMode::Darken => (*a).min(*b),
                MaskMode::Difference => (*a - *b).abs(),
                MaskMode::None => *a,
            };
        }
        any = true;
        first = false;
    }
    if !any {
        return None;
    }
    Some(acc)
}

/// Evaluates and rasterizes all enabled masks of `layer` at `frame` into an
/// output-sized RGBA8 coverage texture payload. Returns None when the layer
/// has no effective masks.
/// Evaluates enabled masks into scaled shapes and returns the input cache key.
/// None when the layer has no effective masks this frame.
pub(crate) fn collect_mask_shapes(
    layer: &crate::core::timeline::Layer,
    frame: u32,
    out_w: u32,
    out_h: u32,
    comp_w: u32,
    comp_h: u32,
) -> Option<(u64, Vec<MaskShape>)> {
    use crate::core::mask::MaskMode;
    if layer.masks.is_empty() || out_w == 0 || out_h == 0 {
        return None;
    }
    let sx = out_w as f32 / comp_w.max(1) as f32;
    let sy = out_h as f32 / comp_h.max(1) as f32;
    let mut shapes = Vec::new();
    for mask in &layer.masks {
        if !mask.enabled || mask.mode == MaskMode::None {
            continue;
        }
        let mut poly = mask.path.to_polygon(frame, 12);
        // to_polygon may repeat the first point to close the loop — drop it
        if poly.len() > 1 && poly[0] == *poly.last().unwrap() {
            poly.pop();
        }
        if poly.len() < 3 {
            continue;
        }
        shapes.push(MaskShape {
            poly: poly.iter().map(|p| [p[0] * sx, p[1] * sy]).collect(),
            mode: mask.mode,
            opacity: (mask.opacity.evaluate(frame) / 100.0).clamp(0.0, 1.0),
            inverted: mask.inverted,
            feather: mask.feather.evaluate(frame).max(0.0),
        });
    }
    if shapes.is_empty() {
        return None;
    }
    Some((mask_input_key(&shapes, out_w, out_h), shapes))
}

/// Combines + packs previously collected shapes into the uploadable raster.
/// `key` must come from [`collect_mask_shapes`] for the same inputs.
pub(crate) fn rasterize_from_shapes(
    shapes: &[MaskShape],
    out_w: u32,
    out_h: u32,
) -> Option<MaskRaster> {
    let coverage = combine_mask_shapes(shapes, out_w, out_h)?;
    let mut pixels = vec![0u8; coverage.len() * 4];
    for (i, c) in coverage.iter().enumerate() {
        let a = (c.clamp(0.0, 1.0) * 255.0).round() as u8;
        pixels[i * 4] = 255;
        pixels[i * 4 + 1] = 255;
        pixels[i * 4 + 2] = 255;
        pixels[i * 4 + 3] = a;
    }
    Some(MaskRaster {
        key: mask_input_key(shapes, out_w, out_h),
        pixels,
    })
}

#[cfg(test)]
include!("mask_raster/tests.rs");
