#[test]
fn parallel_render_mfr_survives_concurrent_cancel_without_overcounting() {
    use std::sync::atomic::AtomicBool;
    use std::sync::Barrier;
    use std::thread;

    let mut queue = ParallelRenderQueue::new();
    for item in 0..8 {
        queue.add_item(RenderQueueItem {
            comp_name: format!("comp_{item}"),
            start_frame: 0,
            end_frame: 63,
            output_path: format!("/tmp/stress_{item}.png"),
            status: kagari_vfx::core::parallel_render::RenderStatus::Pending,
        });
    }
    let external_cancel = Arc::new(AtomicBool::new(false));
    let first_frame_started = Arc::new(Barrier::new(2));
    let cancel_for_thread = Arc::clone(&external_cancel);
    let barrier_for_thread = Arc::clone(&first_frame_started);
    let canceller = thread::spawn(move || {
        barrier_for_thread.wait();
        cancel_for_thread.store(true, Ordering::SeqCst);
    });
    let callback_barrier = Arc::clone(&first_frame_started);
    let callback_started = Arc::new(AtomicBool::new(false));
    let callback_started_once = Arc::clone(&callback_started);

    queue.render_all_mfr_with_external_cancel(&external_cancel, move |_, _| {
        if !callback_started_once.swap(true, Ordering::SeqCst) {
            callback_barrier.wait();
        }
        thread::yield_now();
        vec![0u8; 4]
    });
    canceller.join().unwrap();

    let rendered = queue.total_frames_rendered.load(Ordering::SeqCst);
    assert!(
        queue.is_cancelled(),
        "worker cancellation must be observable"
    );
    assert!(
        rendered <= queue.total_frames,
        "cancelled render overcounted frames: {rendered} > {}",
        queue.total_frames
    );
}
#[test]
fn render_stats_calculation() {
    let stats = RenderStats {
        frames_rendered: 100,
        total_frames: 200,
        elapsed_ms: 5000.0,
        avg_frame_ms: 50.0,
        active_threads: 8,
    };
    assert!((stats.progress_pct() - 50.0).abs() < 0.01);
    assert!((stats.fps() - 20.0).abs() < 0.1);
}

// ─────────────────────────────────────────────────────────────────────────────
// §5  Effect Parameter Cross-Module Consistency
//     Every EffectType variant must render with zeroed/constant parameters
//     without panic or infinite loop.
// ─────────────────────────────────────────────────────────────────────────────

fn make_pixels(w: u32, h: u32) -> Vec<u8> {
    let mut p = vec![128u8; (w * h * 4) as usize];
    // Add gradient to avoid trivial zero-input optimization
    for (i, px) in p.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        px[0] = (i as f32 / (w * h) as f32 * 255.0) as u8;
        px[1] = 128;
        px[2] = 64;
        px[3] = 255;
    }
    p
}
#[test]
fn all_effect_variants_render_constant_params() {
    let w = 32u32;
    let h = 32u32;
    let effects: Vec<(&str, Vec<Effect>)> = vec![
        (
            "GaussianBlur",
            vec![fx(EffectType::GaussianBlur {
                blur_radius: c32(3.0),
            })],
        ),
        (
            "ColorTint",
            vec![fx(EffectType::ColorTint {
                color: c32a4([1.0, 0.0, 0.0, 1.0]),
                intensity: c32(50.0),
            })],
        ),
        (
            "DropShadow",
            vec![fx(EffectType::DropShadow {
                color: c32a4([0.0, 0.0, 0.0, 1.0]),
                opacity: c32(75.0),
                direction: c32(135.0),
                distance: c32(10.0),
                softness: c32(5.0),
            })],
        ),
        (
            "ChromaticAberration",
            vec![fx(EffectType::ChromaticAberration {
                shift_r: c32(2.0),
                shift_b: c32(-2.0),
                edge_falloff: c32(0.5),
                iris_linked: false,
            })],
        ),
        (
            "Vignette",
            vec![fx(EffectType::Vignette {
                intensity: c32(50.0),
                roundness: c32(0.5),
                feather: c32(50.0),
                color: c32a4([0.0, 0.0, 0.0, 1.0]),
            })],
        ),
        (
            "Levels",
            vec![fx(EffectType::Levels {
                input_black: c32(0.0),
                input_white: c32(1.0),
                gamma: c32(1.0),
                output_black: c32(0.0),
                output_white: c32(1.0),
            })],
        ),
        (
            "HueSaturation",
            vec![fx(EffectType::HueSaturation {
                hue_shift: c32(0.0),
                saturation: c32(1.0),
                lightness: c32(0.0),
            })],
        ),
        (
            "Glow",
            vec![fx(EffectType::Glow {
                threshold: c32(0.8),
                radius: c32(10.0),
                intensity: c32(1.5),
                color: c32a4([1.0, 1.0, 1.0, 1.0]),
            })],
        ),
        (
            "Twirl",
            vec![fx(EffectType::Twirl {
                angle: c32(45.0),
                radius: c32(50.0),
            })],
        ),
        (
            "Bulge",
            vec![fx(EffectType::Bulge {
                amount: c32(0.5),
                radius: c32(100.0),
            })],
        ),
        (
            "Posterize",
            vec![fx(EffectType::Posterize { levels: c32(8.0) })],
        ),
        (
            "Invert",
            vec![fx(EffectType::Invert {
                invert_alpha: false,
            })],
        ),
        (
            "Sharpen",
            vec![fx(EffectType::Sharpen { amount: c32(50.0) })],
        ),
        (
            "Threshold",
            vec![fx(EffectType::Threshold {
                threshold: c32(128.0),
            })],
        ),
        (
            "MotionBlur",
            vec![fx(EffectType::MotionBlur {
                shutter_angle: c32(180.0),
                samples: 8,
            })],
        ),
        (
            "FilmGrain",
            vec![fx(EffectType::FilmGrain {
                intensity: c32(0.3),
                grain_size: 1.5,
                color_film: false,
            })],
        ),
        (
            "DirectionalBlur",
            vec![fx(EffectType::DirectionalBlur {
                angle: c32(45.0),
                length: c32(10.0),
            })],
        ),
        (
            "RadialBlur",
            vec![fx(EffectType::RadialBlur { amount: c32(10.0) })],
        ),
        (
            "LinearWipe",
            vec![fx(EffectType::LinearWipe {
                completion: c32(50.0),
                angle: c32(0.0),
            })],
        ),
        (
            "Offset",
            vec![fx(EffectType::Offset {
                shift_x: c32(10.0),
                shift_y: c32(10.0),
            })],
        ),
        (
            "SimpleChoker",
            vec![fx(EffectType::SimpleChoker {
                choke_amount: c32(2.0),
            })],
        ),
        (
            "TurbulentDisplace",
            vec![fx(EffectType::TurbulentDisplace {
                amount: c32(20.0),
                size: c32(50.0),
                evolution: c32(0.0),
                complexity: c32(3.0),
            })],
        ),
        (
            "Minimax",
            vec![fx(EffectType::Minimax {
                operation: c32(0.5),
                radius: c32(3.0),
            })],
        ),
        (
            "ShiftChannels",
            vec![fx(EffectType::ShiftChannels {
                take_red: c32(0.0),
                take_green: c32(1.0),
                take_blue: c32(2.0),
                take_alpha: c32(3.0),
            })],
        ),
        (
            "VenetianBlinds",
            vec![fx(EffectType::VenetianBlinds {
                completion: c32(50.0),
                width: c32(10.0),
            })],
        ),
        (
            "Tritone",
            vec![fx(EffectType::Tritone {
                shadow_color: Animatable::new_constant([0.0; 3]),
                mid_color: Animatable::new_constant([0.5; 3]),
                highlight_color: Animatable::new_constant([1.0; 3]),
            })],
        ),
        (
            "MatteChoker",
            vec![fx(EffectType::MatteChoker {
                choke_amount: c32(2.0),
                gray_level: c32(0.5),
            })],
        ),
        (
            "Vibrance",
            vec![fx(EffectType::Vibrance { amount: c32(50.0) })],
        ),
        (
            "WhiteBalance",
            vec![fx(EffectType::WhiteBalance {
                temperature: c32(0.0),
                tint: c32(0.0),
            })],
        ),
        (
            "HslAdjust",
            vec![fx(EffectType::HslAdjust {
                hue_deg: c32(0.0),
                saturation: c32(1.0),
                lightness: c32(0.0),
            })],
        ),
        (
            "Vortex",
            vec![fx(EffectType::Vortex {
                radius: c32(100.0),
                angle_deg: c32(45.0),
            })],
        ),
        (
            "HeatDistortion",
            vec![fx(EffectType::HeatDistortion {
                strength: c32(10.0),
                speed: c32(1.0),
            })],
        ),
        (
            "RainRipples",
            vec![fx(EffectType::RainRipples {
                drop_count: c32(10.0),
                wave_strength: c32(0.5),
            })],
        ),
        (
            "Fisheye",
            vec![fx(EffectType::Fisheye { strength: c32(0.5) })],
        ),
        (
            "LensCorrection",
            vec![fx(EffectType::LensCorrection {
                k1: c32(0.0),
                k2: c32(0.0),
            })],
        ),
        (
            "GlitchDisplacement",
            vec![fx(EffectType::GlitchDisplacement {
                seed: c32(42.0),
                amount: c32(10.0),
            })],
        ),
        (
            "CrtScanlines",
            vec![fx(EffectType::CrtScanlines {
                line_spacing: c32(4.0),
                intensity: c32(0.5),
            })],
        ),
        (
            "GlowPro",
            vec![fx(EffectType::GlowPro {
                threshold: c32(0.8),
                radius: c32(10.0),
                intensity: c32(1.5),
            })],
        ),
        (
            "RadialFastBlur",
            vec![fx(EffectType::RadialFastBlur {
                amount: c32(10.0),
                samples: 8,
            })],
        ),
        (
            "BendIt",
            vec![fx(EffectType::BendIt {
                top_offset: c32(0.0),
                bottom_offset: c32(0.0),
            })],
        ),
        (
            "Tiler",
            vec![fx(EffectType::Tiler {
                scale_percent: c32(100.0),
                mirror: false,
            })],
        ),
    ];

    for (name, effects) in &effects {
        let mut pixels = make_pixels(w, h);
        let mut repeat = make_pixels(w, h);
        kagari_vfx::core::cpu_effects::apply_layer_effects(
            None,
            None,
            &mut pixels,
            w,
            h,
            effects,
            0,
            30,
        );
        kagari_vfx::core::cpu_effects::apply_layer_effects(
            None,
            None,
            &mut repeat,
            w,
            h,
            effects,
            0,
            30,
        );
        // Verify buffer size unchanged
        assert_eq!(
            pixels.len(),
            (w * h * 4) as usize,
            "{name}: wrong buffer size"
        );
        assert_eq!(pixels, repeat, "{name}: same input must render identically");
    }
}
#[test]
fn representative_effects_change_pixels_semantically() {
    let input = vec![64, 128, 192, 255];

    let mut inverted = input.clone();
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None,
        None,
        &mut inverted,
        1,
        1,
        &[fx(EffectType::Invert {
            invert_alpha: false,
        })],
        0,
        30,
    );
    assert_eq!(inverted, vec![191, 127, 63, 255]);

    let mut tinted = input.clone();
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None,
        None,
        &mut tinted,
        1,
        1,
        &[fx(EffectType::ColorTint {
            color: c32a4([1.0, 0.0, 0.0, 1.0]),
            intensity: c32(100.0),
        })],
        0,
        30,
    );
    assert!(tinted[0] > tinted[1]);
    assert!(tinted[1] < input[1]);
    assert_eq!(tinted[3], input[3]);
}
