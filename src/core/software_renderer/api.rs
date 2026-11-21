use super::*;

struct RenderDeadlineWatchdog {
    flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    timer: Option<std::thread::JoinHandle<()>>,
}

impl Drop for RenderDeadlineWatchdog {
    fn drop(&mut self) {
        self.flag.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(timer) = self.timer.take() {
            let _ = timer.join();
        }
    }
}

/// Multi-frame rendering (MFR): render a range of frames in parallel using rayon.
/// Returns a Vec of (frame, pixels) in the same order as the input range.
/// Each frame is rendered independently on a separate CPU core, then the
/// results are collected sequentially for GPU texture upload.
/// Honors the global MFR switch: when disabled the range renders sequentially
/// (identical pixels, single-threaded) so the UI toggle has real effect.
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
    if !crate::core::parallel_render::mfr_enabled() {
        return frames
            .into_iter()
            .map(|f| {
                let pixels = render_frame_to_pixels(comp, f, width, height, exposure_ev, lut_mode);
                (f, pixels)
            })
            .collect();
    }
    let cancel_flag = get_render_cancel_flag();
    frames
        .par_iter()
        .map(|&f| {
            let _cancel_guard = cancel_flag
                .as_ref()
                .map(|flag| RenderCancelFlagGuard::install(flag.clone()));
            let pixels = render_frame_to_pixels(comp, f, width, height, exposure_ev, lut_mode);
            (f, pixels)
        })
        .collect()
}

/// Convert the 8-bit software-renderer output to a linear `f32` buffer.
///
/// This preserves the displayed image in linear-light form, but it is not an HDR
/// rendering path: highlights and precision already lost by the 8-bit renderer
/// cannot be recovered here.
pub fn render_frame_to_hdr_f32(
    comp: &Composition,
    frame: u32,
    width: u32,
    height: u32,
    exposure_ev: f32,
    lut_mode: u32,
) -> crate::core::color_science::HdrF32Buffer {
    let empty = || crate::core::color_science::HdrF32Buffer {
        width: 0,
        height: 0,
        data: Vec::new(),
    };
    if float_render_working_set_size(width, height).is_none() {
        return empty();
    }
    let pixels = render_frame_to_pixels(comp, frame, width, height, exposure_ev, lut_mode);
    if pixels.is_empty() {
        return empty();
    }
    let Some(float_len) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|count| count.checked_mul(4))
    else {
        return empty();
    };
    let mut data = Vec::new();
    if data.try_reserve_exact(float_len).is_err() {
        return empty();
    }
    use crate::core::color::srgb_to_linear_piecewise;
    for pixel in pixels.as_chunks::<4>().0 {
        data.push(srgb_to_linear_piecewise(pixel[0] as f32 / 255.0));
        data.push(srgb_to_linear_piecewise(pixel[1] as f32 / 255.0));
        data.push(srgb_to_linear_piecewise(pixel[2] as f32 / 255.0));
        data.push(pixel[3] as f32 / 255.0);
    }
    crate::core::color_science::HdrF32Buffer {
        width,
        height,
        data,
    }
}

/// Render a frame and convert its 8-bit sRGB result to linear-light f32 RGBA.
///
/// The values represent a linearized 8-bit render and remain in `0.0..=1.0`;
/// this function does not provide a full-precision or HDR composition render.
pub fn render_frame_to_pixels_f32(
    comp: &Composition,
    frame: u32,
    width: u32,
    height: u32,
    exposure_ev: f32,
    lut_mode: u32,
) -> Vec<[f32; 4]> {
    if float_render_working_set_size(width, height).is_none() {
        return Vec::new();
    }
    let buf8 = render_frame_to_pixels(comp, frame, width, height, exposure_ev, lut_mode);
    if buf8.is_empty() {
        return Vec::new();
    }
    let n = (width as usize) * (height as usize);
    let mut out = Vec::new();
    if out.try_reserve_exact(n).is_err() {
        return Vec::new();
    }
    out.resize(n, [0.0f32; 4]);
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

    let start = Instant::now();
    let flag = Arc::new(AtomicBool::new(false));
    let _cancel_guard = RenderCancelFlagGuard::install(flag.clone());

    // Watchdog timer: flips the flag when the deadline expires.
    let timer_flag = flag.clone();
    let timer = std::thread::Builder::new()
        .name("kagari-render-deadline".into())
        .spawn(move || {
            while start.elapsed() < deadline {
                if timer_flag.load(Ordering::Relaxed) {
                    return; // already cancelled by other means
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            timer_flag.store(true, Ordering::Relaxed);
        });
    let timer = match timer {
        Ok(timer) => timer,
        Err(error) => {
            flag.store(true, Ordering::Relaxed);
            log::error!("Could not start render deadline watchdog: {error}");
            drop(_cancel_guard);
            return (Vec::new(), true);
        }
    };
    let watchdog = RenderDeadlineWatchdog {
        flag: flag.clone(),
        timer: Some(timer),
    };

    let pixels = render_frame_to_pixels(comp, frame, width, height, exposure_ev, lut_mode);
    // The watchdog polls at 10 ms intervals, so it may not have published its
    // flag yet when a render returns just after the deadline. Measure elapsed
    // time as well to make the reported result independent of that wakeup race.
    let timed_out = flag.load(Ordering::Relaxed) || start.elapsed() >= deadline;
    drop(watchdog);

    (pixels, timed_out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadline_reports_expiry_even_before_watchdog_poll() {
        let comp = Composition::new("deadline-zero".into(), "Deadline".into(), 8, 8, 24, 1);
        let (pixels, timed_out) =
            render_frame_with_deadline(&comp, 0, 8, 8, 0.0, 0, std::time::Duration::ZERO);

        assert!(timed_out, "zero deadline is expired when rendering returns");
        assert_eq!(pixels.len(), 8 * 8 * 4);
    }

    /// The global MFR switch must change scheduling only — never pixels.
    #[test]
    fn mfr_disabled_range_render_matches_enabled_result() {
        let mut comp = Composition::new("mfr-parity".into(), "MFR Parity".into(), 8, 8, 24, 3);
        let mut layer = crate::core::timeline::Layer::new(
            "mfr-solid".into(),
            "Solid".into(),
            crate::core::timeline::LayerType::Solid {
                color: [0.2, 0.6, 0.9, 1.0],
            },
            24,
        );
        layer.transform.position = crate::core::property::Animatable::new_constant([4.0, 4.0]);
        comp.layers.push(layer);

        let saved = crate::core::parallel_render::mfr_enabled();
        crate::core::parallel_render::set_mfr_enabled(true);
        let parallel = render_frame_range_parallel(&comp, 0, 2, 8, 8, 0.0, 0);
        crate::core::parallel_render::set_mfr_enabled(false);
        let sequential = render_frame_range_parallel(&comp, 0, 2, 8, 8, 0.0, 0);
        crate::core::parallel_render::set_mfr_enabled(saved);

        assert_eq!(sequential.len(), 3, "frames 0..=2 must render");
        assert_eq!(
            parallel, sequential,
            "MFR toggle must change scheduling only, never pixels"
        );
        assert!(
            sequential.iter().any(|(_, px)| px.iter().any(|&v| v > 0)),
            "fixture must actually render something"
        );
    }
}
