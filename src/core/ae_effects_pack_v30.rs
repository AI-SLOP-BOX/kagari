//! After Effects VFX Kernels Part 30 — Temporal Glitch Primitives.
//!
//! Production-quality implementations of:
//!   * RGB Split            — uniform directional per-channel offset (chromatic tear)
//!   * Flicker              — seeded temporal luminance flicker / signal dropout
//!   * Block Glitch         — seeded block displacement + channel corruption
//!   * Slice Tear           — sparse horizontal/vertical slice offsets
//!
//! All functions are deterministic per (seed, frame), panic-free, and
//! byte-identical no-ops when their strength parameters are zero.

/// Integer hash (splitmix-style) for deterministic per-frame randomness.
#[inline]
fn hash_u32(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}

/// Deterministic pseudo-random value in [0, 1) from three u32 lanes.
#[inline]
fn hash01(a: u32, b: u32, c: u32) -> f32 {
    hash_u32(a ^ b.wrapping_mul(0x9E37_79B1) ^ c.wrapping_mul(0x85EB_CA6B)) as f32 / u32::MAX as f32
}

// ──────────────────────────── RGB Split ─────────────────────────────

/// Shift R/G/B channels independently by constant pixel offsets.
///
/// Alpha is taken from the destination pixel (edges stay clean on
/// transparent backgrounds). Nearest-neighbour sampling keeps the crunchy
/// glitch aesthetic. Zero offsets are a byte-identical no-op.
pub fn apply_rgb_split(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    red_offset: [f32; 2],
    green_offset: [f32; 2],
    blue_offset: [f32; 2],
) {
    let rox = red_offset[0].round() as i32;
    let roy = red_offset[1].round() as i32;
    let gox = green_offset[0].round() as i32;
    let goy = green_offset[1].round() as i32;
    let box_ = blue_offset[0].round() as i32;
    let boy = blue_offset[1].round() as i32;
    if rox == 0 && roy == 0 && gox == 0 && goy == 0 && box_ == 0 && boy == 0 {
        return;
    }
    let w = width as i32;
    let h = height as i32;
    if w <= 0 || h <= 0 || pixels.len() < (w as usize) * (h as usize) * 4 {
        return;
    }
    let src = pixels.to_vec();
    let at = |x: i32, y: i32| -> usize {
        let cx = x.clamp(0, w - 1) as usize;
        let cy = y.clamp(0, h - 1) as usize;
        (cy * w as usize + cx) * 4
    };
    for y in 0..h {
        for x in 0..w {
            let dst = at(x, y);
            let sr = at(x - rox, y - roy);
            let sg = at(x - gox, y - goy);
            let sb = at(x - box_, y - boy);
            pixels[dst] = src[sr];
            pixels[dst + 1] = src[sg + 1];
            pixels[dst + 2] = src[sb + 2];
            pixels[dst + 3] = src[dst + 3];
        }
    }
}

// ───────────────────────────── Flicker ──────────────────────────────

/// Temporal luminance flicker: one global multiplier per time step.
///
/// `speed` is pulses per second; the step index advances with
/// `floor(frame * speed / fps)`. At full `amount` a pulse can drop the
/// frame to black (signal dropout). RGB only — alpha untouched.
/// Zero amount (or non-positive speed) is a byte-identical no-op.
pub fn apply_flicker(pixels: &mut [u8], amount: f32, speed: f32, seed: u32, frame: u32, fps: u32) {
    let depth = amount.clamp(0.0, 1.0);
    if depth <= 0.001 || !speed.is_finite() || speed <= 0.0 {
        return;
    }
    let step = ((frame as f32 * speed) / fps.max(1) as f32).floor() as u32;
    let pulse = hash01(seed, step, 0x0F11_C4E4);
    let factor = 1.0 - depth * pulse;
    if factor >= 0.9999 {
        return;
    }
    for chunk in pixels.chunks_exact_mut(4) {
        chunk[0] = (chunk[0] as f32 * factor) as u8;
        chunk[1] = (chunk[1] as f32 * factor) as u8;
        chunk[2] = (chunk[2] as f32 * factor) as u8;
    }
}

// ─────────────────────────── Block Glitch ───────────────────────────

/// Corrupt a seeded subset of grid blocks per frame.
///
/// Each block (with probability `amount`) is copied from a hashed offset
/// (jump grows with `corruption`) and optionally gets its R/B channels
/// swapped when a second hash falls below `corruption`. Block copies move
/// full RGBA quads so straight-alpha data stays valid.
/// Zero amount is a byte-identical no-op.
pub fn apply_block_glitch(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    block_size: f32,
    amount: f32,
    seed: u32,
    frame: u32,
    corruption: f32,
) {
    let density = amount.clamp(0.0, 1.0);
    if density <= 0.001 || !block_size.is_finite() {
        return;
    }
    let bs = (block_size.round() as u32).clamp(1, 512);
    let w = width as usize;
    let h = height as usize;
    if w == 0 || h == 0 || pixels.len() < w * h * 4 {
        return;
    }
    let corr = corruption.clamp(0.0, 1.0);
    let src = pixels.to_vec();
    let blocks_x = width.div_ceil(bs);
    let blocks_y = height.div_ceil(bs);
    let jump = (1.0 + corr * 7.0) * bs as f32;
    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            let lane = by.wrapping_mul(blocks_x).wrapping_add(bx);
            if hash01(seed, frame, lane) >= density {
                continue;
            }
            let dx = ((hash01(seed ^ 0x51AB, frame, lane) - 0.5) * 2.0 * jump).round() as i32;
            let dy = ((hash01(seed ^ 0xBEEF, frame, lane) - 0.5) * jump).round() as i32;
            let scramble = hash01(seed ^ 0x5C9A, frame, lane) < corr;
            let x0 = bx * bs;
            let y0 = by * bs;
            for oy in 0..bs {
                let y = y0 + oy;
                if y >= height {
                    break;
                }
                for ox in 0..bs {
                    let x = x0 + ox;
                    if x >= width {
                        break;
                    }
                    let sx = (x as i32 + dx).clamp(0, width as i32 - 1) as u32;
                    let sy = (y as i32 + dy).clamp(0, height as i32 - 1) as u32;
                    let s = ((sy as usize) * w + sx as usize) * 4;
                    let d = ((y as usize) * w + x as usize) * 4;
                    if scramble {
                        pixels[d] = src[s + 2];
                        pixels[d + 1] = src[s + 1];
                        pixels[d + 2] = src[s];
                        pixels[d + 3] = src[s + 3];
                    } else {
                        pixels[d..d + 4].copy_from_slice(&src[s..s + 4]);
                    }
                }
            }
        }
    }
}

// ──────────────────────────── Slice Tear ────────────────────────────

/// Offset a seeded set of horizontal (or vertical) slices.
///
/// Each of `slices` bands gets a hashed position, thickness and offset up
/// to ±`max_offset` px. Source rows/columns clamp at the edges.
/// Fewer than 1 slice or sub-pixel max offset is a byte-identical no-op.
pub fn apply_slice_tear(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    slices: f32,
    max_offset: f32,
    seed: u32,
    frame: u32,
    vertical: bool,
) {
    if !slices.is_finite() || !max_offset.is_finite() {
        return;
    }
    let n = (slices.floor() as u32).clamp(0, 64);
    if n == 0 || max_offset.abs() < 0.5 || width == 0 || height == 0 {
        return;
    }
    let w = width as usize;
    let h = height as usize;
    if pixels.len() < w * h * 4 {
        return;
    }
    let src = pixels.to_vec();
    // `span` is the axis slices cut across (height for horizontal slices,
    // width for vertical ones); `across` is the axis the offset runs along.
    let span = if vertical { width } else { height };
    let across = if vertical { height } else { width };
    for i in 0..n {
        let p0 = (hash01(seed, frame, i.wrapping_mul(2)) * span as f32) as u32;
        let thick = 1
            + (hash01(seed, frame, i.wrapping_mul(2).wrapping_add(1)) * (span as f32 / 16.0))
                as u32;
        let off = ((hash01(seed ^ 0x71EA, frame, i) - 0.5) * 2.0 * max_offset).round() as i32;
        if off == 0 {
            continue;
        }
        let p1 = (p0 + thick).min(span);
        for p in p0..p1 {
            for q in 0..across {
                let qs = (q as i32 + off).clamp(0, across as i32 - 1) as u32;
                let (sx, sy, dx, dy) = if vertical {
                    (p, qs, p, q)
                } else {
                    (qs, p, q, p)
                };
                let s = ((sy as usize) * w + sx as usize) * 4;
                let d = ((dy as usize) * w + dx as usize) * 4;
                pixels[d..d + 4].copy_from_slice(&src[s..s + 4]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn busy_buf(w: u32, h: u32) -> Vec<u8> {
        let mut v = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                v.push((x.wrapping_mul(37).wrapping_add(y.wrapping_mul(91)) % 251) as u8);
                v.push((x.wrapping_mul(53).wrapping_add(y.wrapping_mul(29)) % 241) as u8);
                v.push((x.wrapping_mul(17).wrapping_add(y.wrapping_mul(67)) % 239) as u8);
                v.push(255);
            }
        }
        v
    }

    #[test]
    fn rgb_split_zero_is_noop() {
        let mut px = busy_buf(16, 16);
        let base = px.clone();
        apply_rgb_split(&mut px, 16, 16, [0.0, 0.0], [0.0, 0.0], [0.0, 0.0]);
        assert_eq!(px, base);
    }

    #[test]
    fn rgb_split_shifts_red_channel() {
        let w = 16;
        let mut px = vec![0u8; (w * 16 * 4) as usize];
        for x in 0..w {
            let i = (x * 4) as usize;
            px[i] = (x * 16) as u8;
            px[i + 1] = 100;
            px[i + 2] = 200;
            px[i + 3] = 255;
        }
        let base = px.clone();
        apply_rgb_split(&mut px, w, 16, [4.0, 0.0], [0.0, 0.0], [0.0, 0.0]);
        assert_ne!(px, base);
        // Row 0, x=8: R sampled from x=4.
        assert_eq!(px[(8 * 4) as usize], base[(4 * 4) as usize]);
        // G/B/alpha untouched.
        assert_eq!(px[(8 * 4 + 1) as usize], 100);
        assert_eq!(px[(8 * 4 + 2) as usize], 200);
        assert_eq!(px[(8 * 4 + 3) as usize], 255);
    }

    #[test]
    fn flicker_zero_is_noop_and_seeded_is_deterministic() {
        let mut px = busy_buf(16, 16);
        let base = px.clone();
        apply_flicker(&mut px, 0.0, 10.0, 7, 30, 30);
        assert_eq!(px, base);
        let mut a = base.clone();
        let mut b = base.clone();
        apply_flicker(&mut a, 0.8, 10.0, 7, 30, 30);
        apply_flicker(&mut b, 0.8, 10.0, 7, 30, 30);
        assert_eq!(a, b);
        assert_ne!(a, base);
        // Alpha channel untouched.
        for chunk in a.chunks_exact(4) {
            assert_eq!(chunk[3], 255);
        }
    }

    #[test]
    fn block_glitch_zero_is_noop_and_seeded_is_deterministic() {
        let mut px = busy_buf(32, 32);
        let base = px.clone();
        apply_block_glitch(&mut px, 32, 32, 8.0, 0.0, 5, 40, 0.8);
        assert_eq!(px, base);
        let mut a = base.clone();
        let mut b = base.clone();
        apply_block_glitch(&mut a, 32, 32, 8.0, 0.9, 5, 40, 0.8);
        apply_block_glitch(&mut b, 32, 32, 8.0, 0.9, 5, 40, 0.8);
        assert_eq!(a, b);
        assert_ne!(a, base);
    }

    #[test]
    fn slice_tear_zero_is_noop_and_seeded_is_deterministic() {
        let mut px = busy_buf(32, 32);
        let base = px.clone();
        apply_slice_tear(&mut px, 32, 32, 0.0, 40.0, 3, 50, false);
        assert_eq!(px, base);
        apply_slice_tear(&mut px, 32, 32, 6.0, 0.0, 3, 50, false);
        assert_eq!(px, base);
        let mut a = base.clone();
        let mut b = base.clone();
        apply_slice_tear(&mut a, 32, 32, 6.0, 20.0, 3, 50, false);
        apply_slice_tear(&mut b, 32, 32, 6.0, 20.0, 3, 50, false);
        assert_eq!(a, b);
        assert_ne!(a, base);
        let mut v = base.clone();
        apply_slice_tear(&mut v, 32, 32, 6.0, 20.0, 3, 50, true);
        assert_ne!(v, base);
    }

    #[test]
    fn glitch_kernels_reject_degenerate_buffers() {
        let mut empty: Vec<u8> = vec![];
        apply_rgb_split(&mut empty, 0, 0, [5.0, 0.0], [0.0, 0.0], [0.0, 0.0]);
        apply_block_glitch(&mut empty, 0, 0, 8.0, 1.0, 1, 1, 1.0);
        apply_slice_tear(&mut empty, 0, 0, 4.0, 9.0, 1, 1, false);
        apply_flicker(&mut empty, 1.0, 10.0, 1, 1, 30);
        assert!(empty.is_empty());
    }
}
