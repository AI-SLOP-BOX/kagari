//! OBS-Studio inspired plugin trait system for render effects.
//!
//! # Current Architecture (Adapter Pattern)
//! `EffectParams` is a flat struct that mirrors the `LayerUniform` GPU buffer.
//! `evaluate_effects()` acts as an adapter: it iterates `EffectType` enum variants
//! and writes the results into the flat `EffectParams`, which is then uploaded to the GPU.
//!
//! # Future Roadmap (True Plugin Architecture)
//! The goal is to evolve toward a system where each plugin carries its own
//! WGSL shader fragment and an arbitrary GPU buffer, enabling multi-pass
//! compositing without touching the core renderer.

#![allow(dead_code)]

pub mod adapter;
pub mod naming;
pub mod params;
pub mod registry;

pub use adapter::{
    evaluate_effects, gpu_preview_effect_supported, EnumEffectPlugin, RenderEffectPlugin,
};
pub use naming::{effect_name, effect_type_id};
pub use params::EffectParams;
pub use registry::{CustomPixelEffectPlugin, CustomPluginRegistry, EffectPluginRegistry};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::timeline::EffectType;

    struct InvertColorPlugin;
    impl CustomPixelEffectPlugin for InvertColorPlugin {
        fn id(&self) -> &str {
            "vendor.invert"
        }
        fn name(&self) -> &str {
            "Vendor Invert Colors"
        }
        fn process_pixels(
            &self,
            pixels: &mut [u8],
            _width: u32,
            _height: u32,
            _frame: u32,
            _time: f32,
        ) {
            for chunk in pixels.as_chunks_mut::<4>().0 {
                chunk[0] = 255 - chunk[0];
                chunk[1] = 255 - chunk[1];
                chunk[2] = 255 - chunk[2];
            }
        }
    }

    #[test]
    fn test_custom_plugin_registration_and_execution() {
        let registry = CustomPluginRegistry::new();
        registry.register(Box::new(InvertColorPlugin));

        let list = registry.list_plugins();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].0, "vendor.invert");

        let mut pixels = vec![100, 150, 200, 255];
        assert!(registry.process_layer("vendor.invert", &mut pixels, 1, 1, 0, 0.0));
        assert_eq!(pixels, vec![155, 105, 55, 255]);
    }

    #[test]
    fn gpu_preview_support_rejects_effects_without_shader_mapping() {
        let invert = EffectType::Invert {
            invert_alpha: false,
        };
        let lut = EffectType::ColorGradeLUT {
            lut_path: "missing.cube".to_string(),
            intensity: crate::core::property::Animatable::new_constant(100.0),
        };

        assert!(gpu_preview_effect_supported(&invert));
        assert!(!gpu_preview_effect_supported(&lut));
    }

    #[test]
    fn gpu_preview_falls_back_when_shader_semantics_diverge_from_export() {
        use crate::core::property::Animatable;

        fn constant<T: Clone>(value: T) -> Animatable<T> {
            Animatable::new_constant(value)
        }
        let unsupported = [
            EffectType::MotionBlur {
                shutter_angle: constant(180.0),
                samples: 16,
            },
            EffectType::FilmGrain {
                intensity: constant(50.0),
                grain_size: 2.0,
                color_film: true,
            },
            EffectType::FractalNoise {
                fractal_type: constant(1.0),
                contrast: constant(100.0),
                brightness: constant(0.0),
                complexity: constant(5.0),
                evolution: constant(0.0),
            },
            EffectType::Tritone {
                shadow_color: constant([0.0, 0.0, 0.0]),
                mid_color: constant([0.5, 0.5, 0.5]),
                highlight_color: constant([1.0, 1.0, 1.0]),
            },
            EffectType::GlowPro {
                threshold: constant(0.8),
                radius: constant(20.0),
                intensity: constant(2.0),
            },
            EffectType::Glow {
                threshold: constant(80.0),
                radius: constant(20.0),
                intensity: constant(100.0),
                color: constant([1.0, 1.0, 1.0, 1.0]),
            },
            EffectType::HueSaturation {
                hue_shift: constant(45.0),
                saturation: constant(20.0),
                lightness: constant(0.0),
            },
            EffectType::GaussianBlur {
                blur_radius: constant(8.0),
            },
            EffectType::DropShadow {
                color: constant([0.0, 0.0, 0.0, 1.0]),
                opacity: constant(75.0),
                direction: constant(135.0),
                distance: constant(8.0),
                softness: constant(10.0),
            },
            EffectType::ChromaticAberration {
                shift_r: constant(4.0),
                shift_b: constant(4.0),
                edge_falloff: constant(1.0),
                iris_linked: false,
            },
            EffectType::Vignette {
                intensity: constant(50.0),
                roundness: constant(0.5),
                feather: constant(50.0),
                color: constant([0.0, 0.0, 0.0, 1.0]),
            },
            EffectType::MeshWarp {
                top_left: constant([0.0, 0.0]),
                top_right: constant([1.0, 0.0]),
                bottom_left: constant([0.0, 1.0]),
                bottom_right: constant([1.0, 1.0]),
            },
            EffectType::Posterize {
                levels: constant(8.0),
            },
            EffectType::CrtScanlines {
                line_spacing: constant(4.0),
                intensity: constant(0.5),
            },
            EffectType::Twirl {
                angle: constant(45.0),
                radius: constant(100.0),
            },
            EffectType::Bulge {
                amount: constant(0.5),
                radius: constant(100.0),
            },
            EffectType::TurbulentDisplace {
                amount: constant(10.0),
                size: constant(5.0),
                evolution: constant(0.0),
                complexity: constant(5.0),
            },
            EffectType::WaveWarp {
                wave_height: constant(10.0),
                wave_width: constant(20.0),
                speed: constant(1.0),
                direction_deg: constant(45.0),
                wave_type: 2,
                pinning: 1,
            },
            EffectType::Spherize {
                radius: constant(100.0),
                refractive_index: constant(1.5),
            },
            EffectType::HeatDistortion {
                strength: constant(20.0),
                speed: constant(1.0),
            },
            EffectType::Invert { invert_alpha: true },
        ];

        assert!(unsupported
            .iter()
            .all(|effect| !gpu_preview_effect_supported(effect)));
        assert!(gpu_preview_effect_supported(&EffectType::ColorTint {
            color: constant([0.2, 0.4, 0.8, 1.0]),
            intensity: constant(50.0),
        }));
        assert!(gpu_preview_effect_supported(&EffectType::Levels {
            input_black: constant(0.1),
            input_white: constant(0.9),
            gamma: constant(1.2),
            output_black: constant(0.05),
            output_white: constant(0.95),
        }));
        assert!(gpu_preview_effect_supported(&EffectType::LensFlare {
            enabled: constant(1.0),
            position_x: constant(0.5),
            position_y: constant(0.5),
            intensity: constant(1.0),
            threshold: constant(1.0),
            color: constant([1.0, 1.0, 1.0, 1.0]),
            link_to_light: None,
        }));
        assert!(!gpu_preview_effect_supported(&EffectType::LensFlare {
            enabled: constant(1.0),
            position_x: constant(0.5),
            position_y: constant(0.5),
            intensity: constant(1.0),
            threshold: constant(1.0),
            color: constant([1.0, 1.0, 1.0, 1.0]),
            link_to_light: Some("key-light".into()),
        }));
        assert!(gpu_preview_effect_supported(&EffectType::Invert {
            invert_alpha: false,
        }));
    }
}
