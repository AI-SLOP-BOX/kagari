use super::*;

/// Multi-frame rendering (MFR): render a range of frames in parallel using rayon.
/// Returns a Vec of (frame, pixels) in the same order as the input range.
/// Each frame is rendered independently on a separate CPU core, then the
/// results are collected sequentially for GPU texture upload.
pub fn render_frame_range_parallel(
    comp: &Composition,
    from: u32,
    to: u32,
    width: u32,
    height: u32,
    exposure_ev: f32,
    lut_mode: u32,
) -> Vec<(u32, Vec<u8>)> {
    use rayon::prelude::*;

    let frames: Vec<u32> = (from..=to).collect();
    frames
        .par_iter()
        .map(|&f| {
            let pixels = render_frame_to_pixels(comp, f, width, height, exposure_ev, lut_mode);
            (f, pixels)
        })
        .collect()
}

/// Render a frame directly into a 32bpc scene-linear HDR float buffer (`HdrF32Buffer`).
///
/// Pixels are stored as 32-bit floats [R, G, B, A] in unbounded scene-linear space,
/// preserving over-bright highlights (>1.0) and negative deep shadows for EXR / HDR export.
pub fn render_frame_to_hdr_f32(
    comp: &Composition,
    frame: u32,
    width: u32,
    height: u32,
    exposure_ev: f32,
    lut_mode: u32,
) -> crate::core::color_science::HdrF32Buffer {
    let f32_pixels = render_frame_to_pixels_f32(comp, frame, width, height, exposure_ev, lut_mode);
    let mut hdr = crate::core::color_science::HdrF32Buffer::new(width, height);
    for (i, px) in f32_pixels.iter().enumerate() {
        let base = i * 4;
        if base + 3 < hdr.data.len() {
            hdr.data[base] = px[0];
            hdr.data[base + 1] = px[1];
            hdr.data[base + 2] = px[2];
            hdr.data[base + 3] = px[3];
        }
    }
    hdr
}

/// Render a frame and return linear-light f32 RGBA pixels (16/32bpc path).
///
/// Returns physically-linear colour values [r, g, b, a] in `0.0..=∞`, suitable
/// for OpenEXR / HDR export, ACES processing, and full 32bpc floating point workflows.
pub fn render_frame_to_pixels_f32(
    comp: &Composition,
    frame: u32,
    width: u32,
    height: u32,
    exposure_ev: f32,
    lut_mode: u32,
) -> Vec<[f32; 4]> {
    let buf8 = render_frame_to_pixels(comp, frame, width, height, exposure_ev, lut_mode);
    if buf8.is_empty() {
        return Vec::new();
    }
    let n = (width as usize) * (height as usize);
    let mut out = vec![[0.0f32; 4]; n];
    use crate::core::color::srgb_to_linear_piecewise;
    for (px, dst) in buf8.as_chunks::<4>().0.iter().zip(out.iter_mut()) {
        // Decode the sRGB-encoded result back to linear (exposure baked in).
        let r = srgb_to_linear_piecewise(px[0] as f32 / 255.0);
        let g = srgb_to_linear_piecewise(px[1] as f32 / 255.0);
        let b = srgb_to_linear_piecewise(px[2] as f32 / 255.0);
        let a = px[3] as f32 / 255.0;
        *dst = [r, g, b, a];
    }
    out
}

/// Renders a frame with a hard deadline. If rendering exceeds `deadline`, the
/// cooperative cancel flag trips and the function returns early with
/// `timed_out = true` and whatever was composited so far.
///
/// This is the watchdog primitive for hang protection: wrap risky renders so a
/// pathological composition degrades to an incomplete frame instead of freezing
/// the export pipeline or UI thread forever.
pub fn render_frame_with_deadline(
    comp: &Composition,
    frame: u32,
    width: u32,
    height: u32,
    exposure_ev: f32,
    lut_mode: u32,
    deadline: std::time::Duration,
) -> (Vec<u8>, bool) {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Instant;

    let flag = Arc::new(AtomicBool::new(false));
    set_render_cancel_flag(Some(flag.clone()));

    // Watchdog timer: flips the flag when the deadline expires.
    let timer_flag = flag.clone();
    let timer = std::thread::spawn(move || {
        let start = Instant::now();
        while start.elapsed() < deadline {
            if timer_flag.load(Ordering::Relaxed) {
                return; // already cancelled by other means
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        timer_flag.store(true, Ordering::Relaxed);
    });

    let pixels = render_frame_to_pixels(comp, frame, width, height, exposure_ev, lut_mode);
    let timed_out = flag.load(Ordering::Relaxed);

    set_render_cancel_flag(None);

    // Stop the watchdog promptly: flipping the flag makes its loop exit.
    flag.store(true, Ordering::Relaxed);
    let _ = timer.join();

    (pixels, timed_out)
}
