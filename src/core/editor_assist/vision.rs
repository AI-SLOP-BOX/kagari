use super::types::{ExposureReport, LetterboxInsets, PixelBounds};

/// Finds the smallest rectangle containing pixels whose alpha exceeds `threshold`.
pub fn transparent_content_bounds(
    rgba: &[u8],
    width: u32,
    height: u32,
    threshold: u8,
) -> Option<PixelBounds> {
    let expected = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    if width == 0 || height == 0 || rgba.len() != expected {
        return None;
    }

    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut found = false;
    for y in 0..height {
        for x in 0..width {
            let alpha = rgba[((y as usize * width as usize + x as usize) * 4) + 3];
            if alpha > threshold {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
                found = true;
            }
        }
    }
    found.then_some(PixelBounds {
        x: min_x,
        y: min_y,
        width: max_x - min_x + 1,
        height: max_y - min_y + 1,
    })
}

/// Detects hard cuts by comparing compact RGB histograms of consecutive frames.
pub fn detect_scene_cuts(
    frames: &[&[u8]],
    width: u32,
    height: u32,
    sensitivity: f32,
) -> Vec<usize> {
    let Some(frame_len) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|v| v.checked_mul(4))
    else {
        return Vec::new();
    };
    if frame_len == 0 || frames.len() < 2 || frames.iter().any(|f| f.len() != frame_len) {
        return Vec::new();
    }
    let threshold = if sensitivity.is_finite() {
        sensitivity.clamp(0.0, 1.0)
    } else {
        0.35
    };
    let histograms: Vec<_> = frames.iter().map(|frame| rgb_histogram(frame)).collect();
    histograms
        .windows(2)
        .enumerate()
        .filter_map(|(index, pair)| {
            let distance = pair[0]
                .iter()
                .zip(&pair[1])
                .map(|(a, b)| a.abs_diff(*b) as f32)
                .sum::<f32>()
                / (2.0 * width as f32 * height as f32 * 3.0);
            (distance >= threshold).then_some(index + 1)
        })
        .collect()
}

fn rgb_histogram(frame: &[u8]) -> [u32; 48] {
    let mut histogram = [0; 48];
    for pixel in frame.as_chunks::<4>().0 {
        histogram[(pixel[0] >> 4) as usize] += 1;
        histogram[16 + (pixel[1] >> 4) as usize] += 1;
        histogram[32 + (pixel[2] >> 4) as usize] += 1;
    }
    histogram
}

/// Finds runs of visually unchanged frames using mean absolute RGB error.
pub fn detect_frozen_frames(
    frames: &[&[u8]],
    width: u32,
    height: u32,
    tolerance: f32,
    minimum_frames: usize,
) -> Vec<std::ops::RangeInclusive<usize>> {
    let Some(frame_len) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|v| v.checked_mul(4))
    else {
        return Vec::new();
    };
    if frame_len == 0
        || frames.len() < 2
        || minimum_frames < 2
        || frames.iter().any(|frame| frame.len() != frame_len)
    {
        return Vec::new();
    }
    let tolerance = if tolerance.is_finite() {
        tolerance.clamp(0.0, 255.0)
    } else {
        0.0
    };
    let mut ranges = Vec::new();
    let mut run_start = 0;
    for index in 1..frames.len() {
        let error = frames[index - 1]
            .as_chunks::<4>()
            .0
            .iter()
            .zip(frames[index].as_chunks::<4>().0.iter())
            .map(|(a, b)| {
                (a[0].abs_diff(b[0]) as f32
                    + a[1].abs_diff(b[1]) as f32
                    + a[2].abs_diff(b[2]) as f32)
                    / 3.0
            })
            .sum::<f32>()
            / (width as f32 * height as f32);
        if error > tolerance {
            if index - run_start >= minimum_frames {
                ranges.push(run_start..=index - 1);
            }
            run_start = index;
        }
    }
    if frames.len() - run_start >= minimum_frames {
        ranges.push(run_start..=frames.len() - 1);
    }
    ranges
}

/// Finds nearly black frames while allowing a small number of bright pixels.
pub fn detect_black_frames(
    frames: &[&[u8]],
    width: u32,
    height: u32,
    luminance_threshold: u8,
    required_dark_fraction: f32,
) -> Vec<usize> {
    let Some(frame_len) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|value| value.checked_mul(4))
    else {
        return Vec::new();
    };
    if frame_len == 0 || frames.iter().any(|frame| frame.len() != frame_len) {
        return Vec::new();
    }
    let required = if required_dark_fraction.is_finite() {
        required_dark_fraction.clamp(0.0, 1.0)
    } else {
        0.99
    };
    frames
        .iter()
        .enumerate()
        .filter_map(|(index, frame)| {
            let dark = frame
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|pixel| {
                    let luminance =
                        (pixel[0] as u32 * 54 + pixel[1] as u32 * 183 + pixel[2] as u32 * 19) / 256;
                    luminance <= luminance_threshold as u32
                })
                .count();
            (dark as f32 / (width as f32 * height as f32) >= required).then_some(index)
        })
        .collect()
}

/// Detects uniformly dark bars touching each edge of a frame.
pub fn detect_letterbox_insets(
    rgba: &[u8],
    width: u32,
    height: u32,
    luminance_threshold: u8,
    required_dark_fraction: f32,
) -> Option<LetterboxInsets> {
    let expected = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    if width == 0 || height == 0 || rgba.len() != expected {
        return None;
    }
    let required = if required_dark_fraction.is_finite() {
        required_dark_fraction.clamp(0.5, 1.0)
    } else {
        0.98
    };
    let row_dark = |y: u32| {
        (0..width)
            .filter(|x| pixel_is_dark(rgba, width, *x, y, luminance_threshold))
            .count() as f32
            / width as f32
            >= required
    };
    let column_dark = |x: u32| {
        (0..height)
            .filter(|y| pixel_is_dark(rgba, width, x, *y, luminance_threshold))
            .count() as f32
            / height as f32
            >= required
    };
    let top = (0..height).take_while(|y| row_dark(*y)).count() as u32;
    let bottom = (0..height).rev().take_while(|y| row_dark(*y)).count() as u32;
    let left = (0..width).take_while(|x| column_dark(*x)).count() as u32;
    let right = (0..width).rev().take_while(|x| column_dark(*x)).count() as u32;
    if top + bottom >= height || left + right >= width {
        return None;
    }
    Some(LetterboxInsets {
        top,
        bottom,
        left,
        right,
    })
}

fn pixel_is_dark(rgba: &[u8], width: u32, x: u32, y: u32, threshold: u8) -> bool {
    let index = (y as usize * width as usize + x as usize) * 4;
    let luminance =
        (rgba[index] as u32 * 54 + rgba[index + 1] as u32 * 183 + rgba[index + 2] as u32 * 19)
            / 256;
    luminance <= threshold as u32
}

/// Reports the fraction of pixels clipped near black and white.
pub fn analyze_exposure_clipping(
    rgba: &[u8],
    width: u32,
    height: u32,
    shadow_threshold: u8,
    highlight_threshold: u8,
) -> Option<ExposureReport> {
    let expected = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    if expected == 0 || rgba.len() != expected || shadow_threshold >= highlight_threshold {
        return None;
    }
    let mut shadows = 0usize;
    let mut highlights = 0usize;
    for pixel in rgba.as_chunks::<4>().0 {
        let minimum = pixel[0].min(pixel[1]).min(pixel[2]);
        let maximum = pixel[0].max(pixel[1]).max(pixel[2]);
        shadows += usize::from(maximum <= shadow_threshold);
        highlights += usize::from(minimum >= highlight_threshold);
    }
    let pixels = width as f32 * height as f32;
    Some(ExposureReport {
        shadow_clipped_fraction: shadows as f32 / pixels,
        highlight_clipped_fraction: highlights as f32 / pixels,
    })
}

/// Finds abrupt one-frame luminance spikes or drops that may indicate flashes.
pub fn detect_flash_frames(
    frames: &[&[u8]],
    width: u32,
    height: u32,
    minimum_luminance_jump: f32,
) -> Vec<usize> {
    let Some(frame_len) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|value| value.checked_mul(4))
    else {
        return Vec::new();
    };
    if frame_len == 0 || frames.len() < 3 || frames.iter().any(|frame| frame.len() != frame_len) {
        return Vec::new();
    }
    let jump = if minimum_luminance_jump.is_finite() {
        minimum_luminance_jump.clamp(0.0, 255.0)
    } else {
        80.0
    };
    let averages: Vec<f32> = frames
        .iter()
        .map(|frame| {
            frame
                .as_chunks::<4>()
                .0
                .iter()
                .map(|pixel| {
                    pixel[0] as f32 * 0.2126 + pixel[1] as f32 * 0.7152 + pixel[2] as f32 * 0.0722
                })
                .sum::<f32>()
                / (width as f32 * height as f32)
        })
        .collect();
    averages
        .windows(3)
        .enumerate()
        .filter_map(|(index, values)| {
            let left = values[1] - values[0];
            let right = values[2] - values[1];
            (left.abs() >= jump && right.abs() >= jump && left.signum() != right.signum())
                .then_some(index + 1)
        })
        .collect()
}

/// Returns frame indices to retain after removing consecutive visual duplicates.
pub fn deduplicate_frame_indices(
    frames: &[&[u8]],
    width: u32,
    height: u32,
    tolerance: f32,
) -> Vec<usize> {
    let Some(frame_len) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|value| value.checked_mul(4))
    else {
        return Vec::new();
    };
    if frame_len == 0 || frames.iter().any(|frame| frame.len() != frame_len) {
        return Vec::new();
    }
    if frames.is_empty() {
        return Vec::new();
    }
    let tolerance = if tolerance.is_finite() {
        tolerance.clamp(0.0, 255.0)
    } else {
        0.0
    };
    let mut retained = vec![0];
    let mut reference = 0;
    for index in 1..frames.len() {
        let error = frames[reference]
            .as_chunks::<4>()
            .0
            .iter()
            .zip(frames[index].as_chunks::<4>().0.iter())
            .map(|(a, b)| {
                (a[0].abs_diff(b[0]) as f32
                    + a[1].abs_diff(b[1]) as f32
                    + a[2].abs_diff(b[2]) as f32)
                    / 3.0
            })
            .sum::<f32>()
            / (width as f32 * height as f32);
        if error > tolerance {
            retained.push(index);
            reference = index;
        }
    }
    retained
}
