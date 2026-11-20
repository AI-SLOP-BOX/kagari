//! HDR Area-Averaged Color Picker & Broadcast Color Scopes:
//! - **Waveform Monitor** (2D horizontal-column vs IRE/linear luma density)
//! - **Vectorscope** (2D Rec.709 `Cb`/`Cr` polar density plot)
//! - **RGB Parade** (R/G/B vertical distributions)

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PickedHdrColor {
    pub linear_rgba: [f32; 4],
    pub srgb_u8: [u8; 4],
    pub luma_709: f32,
    pub stops_over_midgray: f32,
}

#[inline]
fn linear_to_srgb_u8(v: f32) -> u8 {
    let c = v.clamp(0.0, 1.0);
    let s = if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0).round().clamp(0.0, 255.0) as u8
}

/// Samples an area-averaged HDR color over a square `(2*radius+1)^2` around `(cx, cy)`.
pub fn pick_hdr_color_region(
    rgba: &[f32],
    width: u32,
    height: u32,
    cx: u32,
    cy: u32,
    radius: u32,
) -> Option<PickedHdrColor> {
    if width == 0 || height == 0 || rgba.len() != (width as usize) * (height as usize) * 4 {
        return None;
    }
    let r = radius.min(16) as i32;
    let w = width as i32;
    let h = height as i32;
    let mut sum = [0.0_f64; 4];
    let mut cnt = 0.0_f64;

    for dy in -r..=r {
        let y = (cy as i32 + dy).clamp(0, h - 1);
        for dx in -r..=r {
            let x = (cx as i32 + dx).clamp(0, w - 1);
            let i = (y * w + x) as usize * 4;
            for c in 0..4 {
                sum[c] += rgba[i + c] as f64;
            }
            cnt += 1.0;
        }
    }
    let lin = [
        (sum[0] / cnt) as f32,
        (sum[1] / cnt) as f32,
        (sum[2] / cnt) as f32,
        (sum[3] / cnt) as f32,
    ];
    let luma = 0.2126 * lin[0] + 0.7152 * lin[1] + 0.0722 * lin[2];
    let stops = if luma > 1e-5 {
        (luma / 0.18).log2()
    } else {
        -16.0
    };
    Some(PickedHdrColor {
        linear_rgba: lin,
        srgb_u8: [
            linear_to_srgb_u8(lin[0]),
            linear_to_srgb_u8(lin[1]),
            linear_to_srgb_u8(lin[2]),
            (lin[3].clamp(0.0, 1.0) * 255.0).round() as u8,
        ],
        luma_709: luma,
        stops_over_midgray: stops,
    })
}

/// Computes a 2D Luma Waveform density grid (`cols * rows`, row 0 = top/brightest).
pub fn compute_waveform_monitor(
    rgba: &[f32],
    width: u32,
    height: u32,
    cols: usize,
    rows: usize,
) -> Vec<u32> {
    if width == 0 || height == 0 || cols == 0 || rows == 0 || rgba.len() != (width as usize) * (height as usize) * 4 {
        return Vec::new();
    }
    let mut grid = vec![0u32; cols * rows];
    for y in 0..height {
        for x in 0..width {
            let i = (y as usize * width as usize + x as usize) * 4;
            let luma = (0.2126 * rgba[i] + 0.7152 * rgba[i + 1] + 0.0722 * rgba[i + 2]).clamp(0.0, 1.0);
            let col = ((x as usize * cols) / width as usize).min(cols - 1);
            let row = ((1.0 - luma) * (rows - 1) as f32).round() as usize;
            grid[row.min(rows - 1) * cols + col] += 1;
        }
    }
    grid
}

/// Computes a 2D Rec.709 `Cb/Cr` Vectorscope density plot (`size * size`, center = neutral gray).
pub fn compute_vectorscope(rgba: &[f32], width: u32, height: u32, size: usize) -> Vec<u32> {
    if width == 0 || height == 0 || size < 4 || rgba.len() != (width as usize) * (height as usize) * 4 {
        return Vec::new();
    }
    let mut grid = vec![0u32; size * size];
    let half = (size - 1) as f32 * 0.5;
    for px in rgba.chunks_exact(4) {
        let r = px[0].clamp(0.0, 1.0);
        let g = px[1].clamp(0.0, 1.0);
        let b = px[2].clamp(0.0, 1.0);
        let cb = -0.1146 * r - 0.3854 * g + 0.5 * b; // -0.5..=0.5
        let cr = 0.5 * r - 0.4542 * g - 0.0458 * b;  // -0.5..=0.5
        let x = ((cb + 0.5) * 2.0 * half).round().clamp(0.0, (size - 1) as f32) as usize;
        let y = ((0.5 - cr) * 2.0 * half).round().clamp(0.0, (size - 1) as f32) as usize;
        grid[y * size + x] += 1;
    }
    grid
}

/// Computes RGB Parade (`[R, G, B]`, each `cols * rows` density grid).
pub fn compute_rgb_parade(
    rgba: &[f32],
    width: u32,
    height: u32,
    cols: usize,
    rows: usize,
) -> [Vec<u32>; 3] {
    if width == 0 || height == 0 || cols == 0 || rows == 0 || rgba.len() != (width as usize) * (height as usize) * 4 {
        return [Vec::new(), Vec::new(), Vec::new()];
    }
    let mut parades = [
        vec![0u32; cols * rows],
        vec![0u32; cols * rows],
        vec![0u32; cols * rows],
    ];
    for y in 0..height {
        for x in 0..width {
            let i = (y as usize * width as usize + x as usize) * 4;
            let col = ((x as usize * cols) / width as usize).min(cols - 1);
            for ch in 0..3 {
                let v = rgba[i + ch].clamp(0.0, 1.0);
                let row = ((1.0 - v) * (rows - 1) as f32).round() as usize;
                parades[ch][row.min(rows - 1) * cols + col] += 1;
            }
        }
    }
    parades
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hdr_picker_waveform_and_vectorscope_reflect_color_and_luma() {
        // Pure saturated red frame
        let img = [1.0_f32, 0.0, 0.0, 1.0].repeat(16);
        let picked = pick_hdr_color_region(&img, 4, 4, 2, 2, 1).expect("pick");
        assert_eq!(picked.srgb_u8, [255, 0, 0, 255]);
        assert!((picked.luma_709 - 0.2126).abs() < 1e-4);

        let wave = compute_waveform_monitor(&img, 4, 4, 4, 10);
        let expected_row = ((1.0 - 0.2126) * 9.0_f32).round() as usize;
        assert_eq!(wave[expected_row * 4], 4);

        let vec_scope = compute_vectorscope(&img, 4, 4, 17);
        // Pure red has Cr = +0.5 -> top rows (y=0)
        assert!(vec_scope[0..17].iter().sum::<u32>() == 16);
    }
}
