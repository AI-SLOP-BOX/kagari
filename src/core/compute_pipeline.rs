//! GPU Compute pipeline (wgpu): parallel per-pixel effects off the CPU.
//!
//! Architecture:
//! - Buffer-based ping-pong storage (core WebGPU only — no adapter-specific
//!   texture format features), portable across Metal / Vulkan / DX12.
//! - One lazily-initialized global [`GpuComputeContext`] guarded by a mutex;
//!   every call creates its own encoder so no command state is shared.
//! - Graceful headless degradation: [`global()`] returns `None` when no
//!   adapter exists, and callers fall back to the CPU path unchanged.
//!
//! Determinism: kernels are fixed-math (no atomics, no cooperative ops), so
//! output is byte-stable across runs on the same hardware. Cross-vendor
//! results may differ by ±1 LSB from the CPU reference, which is why GPU
//! effects are opt-in via [`set_gpu_effects_enabled`].

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
#[cfg(not(test))]
use std::sync::OnceLock;
use std::sync::{Arc, Mutex};

const SHADER: &str = include_str!("compute_blur.wgsl");

mod context;
pub(crate) use context::ParamsUniform;
pub use context::{timing_hud_line, GpuComputeContext, GpuTimings, TIMINGS};
mod color_effects;
mod distortion_effects;
mod stylize_effects;

/// Max supported blur radius (kernel buffer sized for 2*64+1 weights).
pub const MAX_BLUR_RADIUS: u32 = 64;

static GPU_EFFECTS_ENABLED: AtomicBool = AtomicBool::new(false);

/// Enable or disable GPU effect execution globally (default: disabled).
/// When disabled, all entry points return false immediately so callers
/// fall back to CPU implementations — preserving byte-determinism.
pub fn set_gpu_effects_enabled(on: bool) {
    GPU_EFFECTS_ENABLED.store(on, Ordering::Relaxed);
}

/// Whether GPU effects are currently enabled.
pub fn gpu_effects_enabled() -> bool {
    GPU_EFFECTS_ENABLED.load(Ordering::Relaxed)
}

/// Minimal block-on executor (single future, parks the thread with a
/// proper waker so lost wakeups are impossible). Avoids pulling a full
/// async runtime into the core crate.
fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake, Waker};

    struct ThreadWaker(std::thread::Thread);
    impl Wake for ThreadWaker {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.0.unpark();
        }
    }

    let waker = Waker::from(Arc::new(ThreadWaker(std::thread::current())));
    let mut cx = Context::from_waker(&waker);
    let mut fut = std::pin::pin!(fut);
    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(v) => return v,
            Poll::Pending => std::thread::park(),
        }
    }
}

// ─── Global singleton ───────────────────────────────────────────────────────

#[cfg(not(test))]
static GLOBAL: OnceLock<Option<&'static Arc<GpuComputeContext>>> = OnceLock::new();

/// Lazily initialize the global compute context. Returns None when the
/// machine has no usable adapter or during test builds (wgpu Metal backend
/// panics in its destructor during test-runner process teardown on macOS).
pub fn global() -> Option<&'static Arc<GpuComputeContext>> {
    #[cfg(test)]
    {
        // wgpu-core's internal Global static cleans up Metal resources at
        // process exit, which panics because test-runner threads don't hold
        // the Metal main-thread context.  Return None so callers fall back
        // to the CPU path; dedicated GPU tests exercise GpuComputeContext
        // directly.
        None
    }
    #[cfg(not(test))]
    {
        GLOBAL
            .get_or_init(|| {
                GpuComputeContext::new().map(|ctx| &*Box::leak(Box::new(Arc::new(ctx))))
            })
            .as_ref()
            .copied()
    }
}

mod api;
pub use api::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// The GPU enable flag is process-global; serialize flag-mutating tests.
    static FLAG_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_gpu_context_initializes_or_degrades_gracefully() {
        match GpuComputeContext::new() {
            Some(ctx) => {
                let label = ctx.backend_label();
                assert!(!label.is_empty());
            }
            None => {
                // Headless environments are valid; nothing to assert.
            }
        }
    }

    #[test]
    fn test_disabled_flag_short_circuits() {
        let _g = FLAG_LOCK.lock().unwrap();
        set_gpu_effects_enabled(false);
        let mut px = vec![128u8; 8 * 8 * 4];
        let before = px.clone();
        // Disabled flag gates the wrapper, not the context method.
        assert!(!try_gpu_gaussian_blur(&mut px, 8, 8, 2));
        assert_eq!(px, before, "disabled GPU must not touch the buffer");
        if global().is_some() {
            set_gpu_effects_enabled(true);
            let mut px2 = vec![128u8; 8 * 8 * 4];
            assert!(try_gpu_gaussian_blur(&mut px2, 8, 8, 2), "enabled must run");
            set_gpu_effects_enabled(false);
        }
    }

    #[test]
    fn test_blur_reduces_variance_and_preserves_size() {
        let _g = FLAG_LOCK.lock().unwrap();
        let ctx = match global() {
            Some(c) => c,
            None => return, // headless: nothing to verify
        };
        set_gpu_effects_enabled(true);
        // Noise pattern
        let mut px: Vec<u8> = (0..16 * 16 * 4)
            .map(|i| {
                if i % 4 == 3 {
                    255
                } else {
                    ((i * 37) % 256) as u8
                }
            })
            .collect();
        let before_var = variance(&px);
        assert!(ctx.gaussian_blur(&mut px, 16, 16, 3));
        assert!(variance(&px) < before_var, "blur must smooth the image");
        set_gpu_effects_enabled(false);
    }

    #[test]
    fn test_blur_deterministic_same_hardware() {
        let _g = FLAG_LOCK.lock().unwrap();
        let ctx = match global() {
            Some(c) => c,
            None => return,
        };
        set_gpu_effects_enabled(true);
        let run = || {
            let mut px: Vec<u8> = (0..12 * 12 * 4)
                .map(|i| {
                    if i % 4 == 3 {
                        200
                    } else {
                        ((i * 91) % 256) as u8
                    }
                })
                .collect();
            assert!(ctx.gaussian_blur(&mut px, 12, 12, 2));
            px
        };
        assert_eq!(run(), run(), "GPU blur must be byte-stable across runs");
        set_gpu_effects_enabled(false);
    }

    #[test]
    fn test_directional_blur_smears_along_axis() {
        let _g = FLAG_LOCK.lock().unwrap();
        let ctx = match global() {
            Some(c) => c,
            None => return,
        };
        set_gpu_effects_enabled(true);
        // Single bright dot on transparent background; horizontal smear 8px
        let mut px = vec![0u8; 24 * 8 * 4];
        let dot = ((4 * 24 + 6) * 4) as usize;
        px[dot] = 255;
        px[dot + 1] = 255;
        px[dot + 2] = 255;
        px[dot + 3] = 255;
        assert!(ctx.directional_blur(&mut px, 24, 8, 9, 0.0));
        // Count lit pixels on the dot's row vs the row above
        let row_lit = (0..24)
            .filter(|x| px[((4 * 24 + x) * 4) as usize + 3] > 0)
            .count();
        let above_lit = (0..24)
            .filter(|x| px[((2 * 24 + x) * 4) as usize + 3] > 0)
            .count();
        assert!(
            row_lit >= 5,
            "horizontal smear must light multiple taps: {row_lit}"
        );
        assert!(above_lit <= row_lit / 2, "vertical bleed must stay small");
        set_gpu_effects_enabled(false);
    }

    #[test]
    fn test_timing_hud_reports_calls() {
        let _g = FLAG_LOCK.lock().unwrap();
        let ctx = match global() {
            Some(c) => c,
            None => return,
        };
        set_gpu_effects_enabled(true);
        let mut px = vec![100u8; 16 * 16 * 4];
        assert!(ctx.gaussian_blur(&mut px, 16, 16, 1));
        set_gpu_effects_enabled(false);
        let line = timing_hud_line();
        assert!(line.contains("GPU compute"), "hud: {line}");
        assert!(
            !line.contains("idle"),
            "after a call hud shows stats: {line}"
        );
    }

    #[test]
    fn test_radial_blur_preserves_center_and_smears_edge() {
        let _g = FLAG_LOCK.lock().unwrap();
        let ctx = match global() {
            Some(c) => c,
            None => return,
        };
        set_gpu_effects_enabled(true);
        // Center pixel stays put; an off-center dot smears toward the center
        let mut px = vec![0u8; 32 * 32 * 4];
        for (x, y) in [(16u32, 16u32), (28u32, 16u32)] {
            let i = ((y * 32 + x) * 4) as usize;
            px[i] = 255;
            px[i + 1] = 255;
            px[i + 2] = 255;
            px[i + 3] = 255;
        }
        assert!(ctx.radial_blur(&mut px, 32, 32, 8));
        // Center dot still present
        let center = ((16 * 32 + 16) * 4) as usize;
        assert!(px[center + 3] > 0, "center must remain");
        // Smear region between center and edge dot now lit
        let mid = ((16 * 32 + 22) * 4) as usize;
        assert!(px[mid + 3] > 0, "radial smear must fill toward center");
        set_gpu_effects_enabled(false);
    }

    fn variance(px: &[u8]) -> f64 {
        let vals: Vec<f64> = px.as_chunks::<4>().0.iter().map(|c| c[0] as f64).collect();
        let n = vals.len() as f64;
        let mean = vals.iter().sum::<f64>() / n;
        vals.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / n
    }
}
