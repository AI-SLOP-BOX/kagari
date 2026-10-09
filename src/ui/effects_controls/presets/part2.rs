use crate::core::property::Animatable;
use crate::core::timeline::{Effect, EffectType};
use super::EffectPreset;

pub(crate) fn get_presets_part2() -> Vec<EffectPreset> {
    vec![
        EffectPreset {
            name: "Fractal Noise",
            button_label: "+ Fractal Noise",
            search_key: "fractal noise turbulence procedural texture",
            id_prefix: "fn",
            create_fn: |idx| Effect {
                id: format!("fn_{}", idx),
                name: "Fractal Noise".to_string(),
                effect_type: EffectType::FractalNoise {
                    fractal_type: Animatable::new_constant(0.0),
                    contrast: Animatable::new_constant(100.0),
                    brightness: Animatable::new_constant(0.0),
                    complexity: Animatable::new_constant(5.0),
                    evolution: Animatable::new_constant(0.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Curves",
            button_label: "+ Curves",
            search_key: "curves color correction tone",
            id_prefix: "curves",
            create_fn: |idx| Effect {
                id: format!("curves_{}", idx),
                name: "Curves".to_string(),
                effect_type: EffectType::Curves {
                    channel: Animatable::new_constant(0.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Displacement Map",
            button_label: "+ Displacement Map",
            search_key: "displacement map distortion warp",
            id_prefix: "dispmap",
            create_fn: |idx| Effect {
                id: format!("dispmap_{}", idx),
                name: "Displacement Map".to_string(),
                effect_type: EffectType::DisplacementMap {
                    source_layer: Animatable::new_constant(0.0),
                    max_horizontal: Animatable::new_constant(50.0),
                    max_vertical: Animatable::new_constant(50.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Compound Blur",
            button_label: "+ Compound Blur",
            search_key: "compound blur variable map",
            id_prefix: "cblur",
            create_fn: |idx| Effect {
                id: format!("cblur_{}", idx),
                name: "Compound Blur".to_string(),
                effect_type: EffectType::CompoundBlur {
                    source_layer: Animatable::new_constant(0.0),
                    max_blur: Animatable::new_constant(20.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Minimax",
            button_label: "+ Minimax",
            search_key: "minimax dilate erode matte",
            id_prefix: "mmx",
            create_fn: |idx| Effect {
                id: format!("mmx_{}", idx),
                name: "Minimax".to_string(),
                effect_type: EffectType::Minimax {
                    operation: Animatable::new_constant(0.0),
                    radius: Animatable::new_constant(5.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Shift Channels",
            button_label: "+ Shift Channels",
            search_key: "shift channels swap remap rgba",
            id_prefix: "shiftch",
            create_fn: |idx| Effect {
                id: format!("shiftch_{}", idx),
                name: "Shift Channels".to_string(),
                effect_type: EffectType::ShiftChannels {
                    take_red: Animatable::new_constant(0.0),
                    take_green: Animatable::new_constant(1.0),
                    take_blue: Animatable::new_constant(2.0),
                    take_alpha: Animatable::new_constant(3.0),
                },
                enabled: true,
            },
        },
        // ── Lumetri Basic Correction ──
        // NOTE: names must match the constants in src/ui/lumetri_color.rs so the
        // live sliders in that panel keep driving these same effects.
        EffectPreset {
            name: "Vibrance",
            button_label: "+ Vibrance",
            search_key: "vibrance saturation skin tone lumetri basic correction",
            id_prefix: "lum_vib",
            create_fn: |idx| Effect {
                id: format!("lum_vib_{}", idx),
                name: "Lumetri Vibrance".to_string(),
                effect_type: EffectType::Vibrance {
                    amount: Animatable::new_constant(25.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "White Balance",
            button_label: "+ White Balance",
            search_key: "white balance temperature tint kelvin lumetri basic correction",
            id_prefix: "lum_wb",
            create_fn: |idx| Effect {
                id: format!("lum_wb_{}", idx),
                name: "Lumetri White Balance".to_string(),
                effect_type: EffectType::WhiteBalance {
                    temperature: Animatable::new_constant(0.0),
                    tint: Animatable::new_constant(0.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "HSL Adjust",
            button_label: "+ HSL Adjust",
            search_key: "hsl hue saturation lightness secondary lumetri basic correction",
            id_prefix: "lum_hsl",
            create_fn: |idx| Effect {
                id: format!("lum_hsl_{}", idx),
                name: "Lumetri HSL Adjust".to_string(),
                effect_type: EffectType::HslAdjust {
                    hue_deg: Animatable::new_constant(0.0),
                    saturation: Animatable::new_constant(0.0),
                    lightness: Animatable::new_constant(0.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Glow",
            button_label: "+ Glow (Pro)",
            search_key: "glow bloom threshold bleed stylize light",
            id_prefix: "glowpro",
            create_fn: |idx| Effect {
                id: format!("glowpro_{}", idx),
                name: "Glow".to_string(),
                effect_type: EffectType::GlowPro {
                    threshold: Animatable::new_constant(0.7),
                    radius: Animatable::new_constant(4.0),
                    intensity: Animatable::new_constant(1.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "CRT Scanlines",
            button_label: "+ CRT Scanlines",
            search_key: "crt scanlines tv retro vhs screen",
            id_prefix: "crt",
            create_fn: |idx| Effect {
                id: format!("crt_{}", idx),
                name: "CRT Scanlines".to_string(),
                effect_type: EffectType::CrtScanlines {
                    line_spacing: Animatable::new_constant(3.0),
                    intensity: Animatable::new_constant(0.4),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Vortex Distortion",
            button_label: "+ Vortex",
            search_key: "vortex spiral swirl twist distort",
            id_prefix: "vortex",
            create_fn: |idx| Effect {
                id: format!("vortex_{}", idx),
                name: "Vortex Distortion".to_string(),
                effect_type: EffectType::Vortex {
                    radius: Animatable::new_constant(300.0),
                    angle_deg: Animatable::new_constant(120.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Heat Distortion",
            button_label: "+ Heat Distortion",
            search_key: "heat haze shimmer thermal turbulence fire",
            id_prefix: "heat",
            create_fn: |idx| Effect {
                id: format!("heat_{}", idx),
                name: "Heat Distortion".to_string(),
                effect_type: EffectType::HeatDistortion {
                    strength: Animatable::new_constant(6.0),
                    speed: Animatable::new_constant(1.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Rain Ripples",
            button_label: "+ Rain Ripples",
            search_key: "rain water drop ripple wave puddle",
            id_prefix: "rainrip",
            create_fn: |idx| Effect {
                id: format!("rainrip_{}", idx),
                name: "Rain Ripples".to_string(),
                effect_type: EffectType::RainRipples {
                    drop_count: Animatable::new_constant(12.0),
                    wave_strength: Animatable::new_constant(3.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Fisheye",
            button_label: "+ Fisheye",
            search_key: "fisheye lens bulge round gopro distort",
            id_prefix: "fisheye",
            create_fn: |idx| Effect {
                id: format!("fisheye_{}", idx),
                name: "Fisheye".to_string(),
                effect_type: EffectType::Fisheye {
                    strength: Animatable::new_constant(0.35),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Lens Correction",
            button_label: "+ Lens Correction",
            search_key: "lens correction barrel pincushion camera fix k1 k2",
            id_prefix: "lenscorr",
            create_fn: |idx| Effect {
                id: format!("lenscorr_{}", idx),
                name: "Lens Correction".to_string(),
                effect_type: EffectType::LensCorrection {
                    k1: Animatable::new_constant(0.0),
                    k2: Animatable::new_constant(0.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Glitch Displacement",
            button_label: "+ Glitch",
            search_key: "glitch digital block displacement datamosh vhs error",
            id_prefix: "glitch",
            create_fn: |idx| Effect {
                id: format!("glitch_{}", idx),
                name: "Glitch Displacement".to_string(),
                effect_type: EffectType::GlitchDisplacement {
                    seed: Animatable::new_constant(7.0),
                    amount: Animatable::new_constant(2.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Matte Choke / Spread",
            button_label: "+ Matte Choke",
            search_key: "matte choke spread alpha erode dilate mask edge shrink grow",
            id_prefix: "mchoke",
            create_fn: |idx| Effect {
                id: format!("mchoke_{}", idx),
                name: "Matte Choke / Spread".to_string(),
                effect_type: EffectType::MatteChokeSpread {
                    radius: Animatable::new_constant(3.0),
                    expand: false,
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Alpha Feather",
            button_label: "+ Alpha Feather",
            search_key: "alpha feather soft edge blur mask smooth",
            id_prefix: "afeather",
            create_fn: |idx| Effect {
                id: format!("afeather_{}", idx),
                name: "Alpha Feather".to_string(),
                effect_type: EffectType::AlphaFeather {
                    radius: Animatable::new_constant(4.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Alpha From Luminance",
            button_label: "+ Alpha From Luma",
            search_key: "alpha from luminance luma matte transparency set",
            id_prefix: "aluma",
            create_fn: |idx| Effect {
                id: format!("aluma_{}", idx),
                name: "Alpha From Luminance".to_string(),
                effect_type: EffectType::AlphaFromLuminance { invert: false },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Night Vision",
            button_label: "+ Night Vision",
            search_key: "night vision green phosphor goggles surveillance",
            id_prefix: "nv",
            create_fn: |idx| Effect {
                id: format!("nv_{}", idx),
                name: "Night Vision".to_string(),
                effect_type: EffectType::NightVision {
                    amplification: Animatable::new_constant(2.5),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Iris Wipe",
            button_label: "+ Iris Wipe",
            search_key: "iris circle wipe transition reveal round",
            id_prefix: "irisw",
            create_fn: |idx| Effect {
                id: format!("irisw_{}", idx),
                name: "Iris Wipe".to_string(),
                effect_type: EffectType::IrisWipe {
                    completion: Animatable::new_constant(0.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Radial Wipe",
            button_label: "+ Radial Wipe",
            search_key: "radial sweep wipe transition clock reveal",
            id_prefix: "radw",
            create_fn: |idx| Effect {
                id: format!("radw_{}", idx),
                name: "Radial Wipe".to_string(),
                effect_type: EffectType::RadialWipe {
                    completion: Animatable::new_constant(0.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Film Emulation",
            button_label: "+ Film Emulation",
            search_key: "film emulation kodak fuji cdl lift gamma gain grade look",
            id_prefix: "filmem",
            create_fn: |idx| Effect {
                id: format!("filmem_{}", idx),
                name: "Film Emulation".to_string(),
                effect_type: EffectType::FilmEmulation {
                    lift: Animatable::new_constant(0.0),
                    gamma: Animatable::new_constant(1.0),
                    gain: Animatable::new_constant(1.0),
                    hue_shift_deg: Animatable::new_constant(0.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "God Rays",
            button_label: "+ God Rays",
            search_key: "god rays volumetric light sun scattering beams",
            id_prefix: "godrays",
            create_fn: |idx| Effect {
                id: format!("godrays_{}", idx),
                name: "God Rays".to_string(),
                effect_type: EffectType::GodRays {
                    sun_x: Animatable::new_constant(0.5),
                    sun_y: Animatable::new_constant(0.0),
                    samples: Animatable::new_constant(24.0),
                    decay: Animatable::new_constant(0.95),
                    weight: Animatable::new_constant(0.6),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Zoom Blur",
            button_label: "+ Zoom Blur",
            search_key: "zoom blur radial motion speed warp center",
            id_prefix: "zblur",
            create_fn: |idx| Effect {
                id: format!("zblur_{}", idx),
                name: "Zoom Blur".to_string(),
                effect_type: EffectType::RadialBlurZoom {
                    amount: Animatable::new_constant(20.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Median Filter",
            button_label: "+ Median Filter",
            search_key: "median filter noise removal salt pepper denoise",
            id_prefix: "medf",
            create_fn: |idx| Effect {
                id: format!("medf_{}", idx),
                name: "Median Filter".to_string(),
                effect_type: EffectType::MedianFilter {
                    radius: Animatable::new_constant(2.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Sobel Edges",
            button_label: "+ Sobel Edges",
            search_key: "sobel edge detection outline sketch line",
            id_prefix: "sobel",
            create_fn: |idx| Effect {
                id: format!("sobel_{}", idx),
                name: "Sobel Edges".to_string(),
                effect_type: EffectType::SobelEdges { invert: false },
                enabled: true,
            },
        },

    ]
}
