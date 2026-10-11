use super::*;
use crate::core::property::Animatable;
use crate::core::timeline::{
    BlendMode, Composition, Effect, EffectType, Layer, LayerType, ShapeType, TrackMatteMode,
};

fn write_gray_png(path: &std::path::Path, gray: u8) {
    let img = image::GrayImage::from_pixel(4, 4, image::Luma([gray]));
    image::DynamicImage::ImageLuma8(img)
        .save_with_format(path, image::ImageFormat::Png)
        .expect("save png");
}

fn write_gray_webp(path: &std::path::Path, gray: u8) {
    let img = image::GrayImage::from_pixel(4, 4, image::Luma([gray]));
    image::DynamicImage::ImageLuma8(img)
        .save_with_format(path, image::ImageFormat::WebP)
        .expect("save webp");
}

include!("tests/cases_01.rs");
include!("tests/cases_02.rs");
include!("tests/cases_03.rs");

#[cfg(test)]
mod render_size_guard_tests {
    use super::*;
    use crate::core::timeline::{Composition, Layer, LayerType};

    fn tiny_comp() -> Composition {
        let mut comp = Composition::new("c".into(), "Guard".into(), 32, 32, 30, 30);
        let l = Layer::new(
            "l".into(),
            "S".into(),
            LayerType::Solid { color: [1.0; 4] },
            30,
        );
        comp.layers.push(l);
        comp
    }

    #[test]
    fn test_zero_and_huge_dimensions_return_empty() {
        let comp = tiny_comp();
        // Zero dimensions
        assert!(render_frame_to_pixels(&comp, 0, 0, 32, 0.0, 0).is_empty());
        assert!(render_frame_to_pixels(&comp, 0, 32, 0, 0.0, 0).is_empty());
        // Huge dimensions (would be ~40 GB) must be rejected without allocating
        assert!(render_frame_to_pixels(&comp, 0, 100_000, 100_000, 0.0, 0).is_empty());
        assert!(render_frame_to_pixels(&comp, 0, u32::MAX, u32::MAX, 0.0, 0).is_empty());
    }

    #[test]
    fn test_max_dimension_still_renders_small() {
        let comp = tiny_comp();
        let pixels = render_frame_to_pixels(&comp, 0, 16, 16, 0.0, 0);
        assert_eq!(pixels.len(), 16 * 16 * 4);
    }

    #[test]
    fn test_rgba_buffer_size_overflow_safe() {
        assert!(rgba_buffer_size(u32::MAX, u32::MAX).is_none());
        assert!(rgba_buffer_size(0, 0).is_none());
        assert_eq!(rgba_buffer_size(2, 2), Some(16));
        assert!(rgba_buffer_size(16384, 16384).is_none());
        assert_eq!(rgba_buffer_size(8192, 8192), Some(8192 * 8192 * 4));
    }
}

#[cfg(test)]
mod cancel_tests {
    use super::*;
    use crate::core::timeline::{Composition, Layer, LayerType};
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    fn busy_comp(layers: usize) -> Composition {
        let mut comp = Composition::new("c".into(), "Cancel".into(), 64, 64, 30, 30);
        for i in 0..layers {
            let mut l = Layer::new(
                format!("l{}", i),
                format!("L{}", i),
                LayerType::Solid { color: [0.5; 4] },
                30,
            );
            l.transform.position = crate::core::property::Animatable::new_constant([32.0, 32.0]);
            comp.layers.push(l);
        }
        comp
    }

    #[test]
    fn test_cancel_flag_stops_layer_processing() {
        let comp = busy_comp(200);
        let flag = Arc::new(AtomicBool::new(true)); // cancelled from the start
        set_render_cancel_flag(Some(flag.clone()));

        let pixels = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        // Buffer is still valid; but no layer work should have happened
        assert_eq!(pixels.len(), 64 * 64 * 4);
        // Background-only buffer: no bright solid pixels
        let bright = (0..pixels.len())
            .step_by(4)
            .filter(|&i| pixels[i] > 100)
            .count();
        assert_eq!(bright, 0, "cancelled render must not composite layers");

        set_render_cancel_flag(None);

        // Without the flag the same comp renders layers normally
        let pixels2 = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        let bright2 = (0..pixels2.len())
            .step_by(4)
            .filter(|&i| pixels2[i] > 100)
            .count();
        assert!(bright2 > 0, "un-cancelled render must composite layers");
    }

    #[test]
    fn test_clearing_flag_restores_full_render() {
        let comp = busy_comp(10);
        let flag = Arc::new(AtomicBool::new(false));
        set_render_cancel_flag(Some(flag.clone()));
        set_render_cancel_flag(None); // cleared — must behave as default

        let pixels = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        let bright = (0..pixels.len())
            .step_by(4)
            .filter(|&i| pixels[i] > 100)
            .count();
        assert!(bright > 0);
    }
}

#[cfg(test)]
mod watchdog_tests {
    use super::*;
    use crate::core::timeline::{Composition, Layer, LayerType};

    fn comp(layers: usize) -> Composition {
        let mut c = Composition::new("c".into(), "Watchdog".into(), 64, 64, 30, 30);
        for i in 0..layers {
            let mut l = Layer::new(
                format!("l{}", i),
                format!("L{}", i),
                LayerType::Solid { color: [0.6; 4] },
                30,
            );
            l.transform.position = crate::core::property::Animatable::new_constant([32.0, 32.0]);
            c.layers.push(l);
        }
        c
    }

    #[test]
    fn test_fast_render_completes_without_timeout() {
        let (pixels, timed_out) = render_frame_with_deadline(
            &comp(5),
            0,
            64,
            64,
            0.0,
            0,
            std::time::Duration::from_secs(5),
        );
        assert!(!timed_out);
        assert_eq!(pixels.len(), 64 * 64 * 4);
        // Layers must actually be composited
        let bright = (0..pixels.len())
            .step_by(4)
            .filter(|&i| pixels[i] > 100)
            .count();
        assert!(bright > 0);
    }

    #[test]
    fn test_deadline_aborts_render_early() {
        // Generous layer count; deadline so short the render cannot finish
        let start = std::time::Instant::now();
        let (pixels, timed_out) = render_frame_with_deadline(
            &comp(2000),
            0,
            256,
            256,
            0.0,
            0,
            std::time::Duration::from_millis(5),
        );
        let elapsed = start.elapsed();
        assert!(timed_out, "render must report timeout");
        assert!(
            elapsed < std::time::Duration::from_millis(2000),
            "watchdog must return promptly, took {:?}",
            elapsed
        );
        // Partial buffer is still a valid allocation
        assert_eq!(pixels.len(), 256 * 256 * 4);
    }

    #[test]
    fn test_watchdog_does_not_leak_cancel_state() {
        // After a timed-out render, normal renders must work again
        let _ = render_frame_with_deadline(
            &comp(500),
            0,
            128,
            128,
            0.0,
            0,
            std::time::Duration::from_millis(1),
        );
        let pixels = render_frame_to_pixels(&comp(3), 0, 64, 64, 0.0, 0);
        let bright = (0..pixels.len())
            .step_by(4)
            .filter(|&i| pixels[i] > 100)
            .count();
        assert!(
            bright > 0,
            "cancel state must not leak into subsequent renders"
        );
    }
}

#[cfg(test)]
#[path = "shadow_tests.rs"]
mod shadow_tests;
