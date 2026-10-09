use crate::core::property::Animatable;
use crate::core::timeline::{ColorConversionMode, Effect, EffectType};
use super::EffectPreset;

pub(crate) fn get_presets_part1() -> Vec<EffectPreset> {
    vec![
        EffectPreset {
            name: "Gaussian Blur",
            button_label: "+ Gaussian Blur",
            search_key: "gaussian blur",
            id_prefix: "blur",
            create_fn: |idx| Effect {
                id: format!("blur_{}", idx),
                name: "Gaussian Blur".to_string(),
                effect_type: EffectType::GaussianBlur {
                    blur_radius: Animatable::new_constant(10.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Color Tint",
            button_label: "+ Color Tint",
            search_key: "color tint",
            id_prefix: "tint",
            create_fn: |idx| Effect {
                id: format!("tint_{}", idx),
                name: "Color Tint".to_string(),
                effect_type: EffectType::ColorTint {
                    color: Animatable::new_constant([1.0, 0.2, 0.2, 1.0]),
                    intensity: Animatable::new_constant(50.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Drop Shadow",
            button_label: "+ Drop Shadow",
            search_key: "drop shadow",
            id_prefix: "shadow",
            create_fn: |idx| Effect {
                id: format!("shadow_{}", idx),
                name: "Drop Shadow".to_string(),
                effect_type: EffectType::DropShadow {
                    color: Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
                    opacity: Animatable::new_constant(50.0),
                    direction: Animatable::new_constant(135.0),
                    distance: Animatable::new_constant(5.0),
                    softness: Animatable::new_constant(5.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Chromatic Aberration",
            button_label: "+ Chromatic Aberration",
            search_key: "chromatic aberration",
            id_prefix: "ca",
            create_fn: |idx| Effect {
                id: format!("ca_{}", idx),
                name: "Chromatic Aberration".to_string(),
                effect_type: EffectType::ChromaticAberration {
                    shift_r: Animatable::new_constant(3.0),
                    shift_b: Animatable::new_constant(3.0),
                    edge_falloff: Animatable::new_constant(0.5),
                    iris_linked: true,
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Vignette",
            button_label: "+ Vignette",
            search_key: "vignette",
            id_prefix: "vignette",
            create_fn: |idx| Effect {
                id: format!("vignette_{}", idx),
                name: "Vignette".to_string(),
                effect_type: EffectType::Vignette {
                    intensity: Animatable::new_constant(60.0),
                    roundness: Animatable::new_constant(1.0),
                    feather: Animatable::new_constant(50.0),
                    color: Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Levels",
            button_label: "+ Levels (Gamma/Crush)",
            search_key: "levels",
            id_prefix: "levels",
            create_fn: |idx| Effect {
                id: format!("levels_{}", idx),
                name: "Levels".to_string(),
                effect_type: EffectType::Levels {
                    input_black: Animatable::new_constant(0.0),
                    input_white: Animatable::new_constant(1.0),
                    gamma: Animatable::new_constant(1.0),
                    output_black: Animatable::new_constant(0.0),
                    output_white: Animatable::new_constant(1.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Hue / Saturation",
            button_label: "+ Hue / Saturation",
            search_key: "hue / saturation",
            id_prefix: "huesat",
            create_fn: |idx| Effect {
                id: format!("huesat_{}", idx),
                name: "Hue / Saturation".to_string(),
                effect_type: EffectType::HueSaturation {
                    hue_shift: Animatable::new_constant(0.0),
                    saturation: Animatable::new_constant(0.0),
                    lightness: Animatable::new_constant(0.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Glow",
            button_label: "+ Glow",
            search_key: "glow",
            id_prefix: "glow",
            create_fn: |idx| Effect {
                id: format!("glow_{}", idx),
                name: "Glow".to_string(),
                effect_type: EffectType::Glow {
                    threshold: Animatable::new_constant(50.0),
                    radius: Animatable::new_constant(20.0),
                    intensity: Animatable::new_constant(50.0),
                    color: Animatable::new_constant([1.0, 1.0, 1.0, 1.0]),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Mesh Warp",
            button_label: "+ Mesh Warp (Grid)",
            search_key: "mesh warp",
            id_prefix: "meshwarp",
            create_fn: |idx| Effect {
                id: format!("meshwarp_{}", idx),
                name: "Mesh Warp".to_string(),
                effect_type: EffectType::MeshWarp {
                    top_left: Animatable::new_constant([0.0, 0.0]),
                    top_right: Animatable::new_constant([1920.0, 0.0]),
                    bottom_left: Animatable::new_constant([0.0, 1080.0]),
                    bottom_right: Animatable::new_constant([1920.0, 1080.0]),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Corner Pin",
            button_label: "+ Corner Pin",
            search_key: "corner pin perspective homography screen insert cc power pin",
            id_prefix: "cornerpin",
            create_fn: |idx| Effect {
                id: format!("cornerpin_{}", idx),
                name: "Corner Pin".to_string(),
                effect_type: EffectType::CornerPin {
                    top_left: Animatable::new_constant([0.0, 0.0]),
                    top_right: Animatable::new_constant([1920.0, 0.0]),
                    bottom_right: Animatable::new_constant([1920.0, 1080.0]),
                    bottom_left: Animatable::new_constant([0.0, 1080.0]),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Cinematic 3D LUT",
            button_label: "+ Cinematic 3D LUT",
            search_key: "lut",
            id_prefix: "lut",
            create_fn: |idx| Effect {
                id: format!("lut_{}", idx),
                name: "Cinematic 3D LUT".to_string(),
                effect_type: EffectType::ColorGradeLUT {
                    lut_path: "alexa_logc_to_rec709.cube".to_string(),
                    intensity: Animatable::new_constant(100.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Color Space Converter",
            button_label: "+ Log Space Converter",
            search_key: "log space converter",
            id_prefix: "convert",
            create_fn: |idx| Effect {
                id: format!("convert_{}", idx),
                name: "Color Space Converter".to_string(),
                effect_type: EffectType::ColorSpaceConvert {
                    mode: ColorConversionMode::LogCToLinear,
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Physical Film Grain",
            button_label: "+ Physical Film Grain",
            search_key: "film grain",
            id_prefix: "grain",
            create_fn: |idx| Effect {
                id: format!("grain_{}", idx),
                name: "Physical Film Grain".to_string(),
                effect_type: EffectType::FilmGrain {
                    intensity: Animatable::new_constant(15.0),
                    grain_size: 1.5,
                    color_film: true,
                },
                enabled: true,
            },
        },
        // ── CPU pixel-effect kernels (core::cpu_effects) ──
        EffectPreset {
            name: "Twirl",
            button_label: "+ Twirl",
            search_key: "twirl swirl rotate distort",
            id_prefix: "twirl",
            create_fn: |idx| Effect {
                id: format!("twirl_{}", idx),
                name: "Twirl".to_string(),
                effect_type: EffectType::Twirl {
                    angle: Animatable::new_constant(90.0),
                    radius: Animatable::new_constant(100.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Bulge",
            button_label: "+ Bulge",
            search_key: "bulge magnify lens distort",
            id_prefix: "bulge",
            create_fn: |idx| Effect {
                id: format!("bulge_{}", idx),
                name: "Bulge".to_string(),
                effect_type: EffectType::Bulge {
                    amount: Animatable::new_constant(30.0),
                    radius: Animatable::new_constant(100.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Posterize",
            button_label: "+ Posterize",
            search_key: "posterize levels poster levels",
            id_prefix: "posterize",
            create_fn: |idx| Effect {
                id: format!("posterize_{}", idx),
                name: "Posterize".to_string(),
                effect_type: EffectType::Posterize {
                    levels: Animatable::new_constant(4.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Invert",
            button_label: "+ Invert",
            search_key: "invert negative reverse",
            id_prefix: "invert",
            create_fn: |idx| Effect {
                id: format!("invert_{}", idx),
                name: "Invert".to_string(),
                effect_type: EffectType::Invert {
                    invert_alpha: false,
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Offset",
            button_label: "+ Offset",
            search_key: "offset shift move translate",
            id_prefix: "offset",
            create_fn: |idx| Effect {
                id: format!("offset_{}", idx),
                name: "Offset".to_string(),
                effect_type: EffectType::Offset {
                    shift_x: Animatable::new_constant(0.0),
                    shift_y: Animatable::new_constant(0.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Directional Blur",
            button_label: "+ Directional Blur",
            search_key: "directional blur motion",
            id_prefix: "dirblur",
            create_fn: |idx| Effect {
                id: format!("dirblur_{}", idx),
                name: "Directional Blur".to_string(),
                effect_type: EffectType::DirectionalBlur {
                    angle: Animatable::new_constant(0.0),
                    length: Animatable::new_constant(10.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Radial Blur",
            button_label: "+ Radial Blur",
            search_key: "radial blur zoom spin",
            id_prefix: "radblur",
            create_fn: |idx| Effect {
                id: format!("radblur_{}", idx),
                name: "Radial Blur".to_string(),
                effect_type: EffectType::RadialBlur {
                    amount: Animatable::new_constant(20.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Sharpen",
            button_label: "+ Sharpen",
            search_key: "sharpen unsharp contrast",
            id_prefix: "sharpen",
            create_fn: |idx| Effect {
                id: format!("sharpen_{}", idx),
                name: "Sharpen".to_string(),
                effect_type: EffectType::Sharpen {
                    amount: Animatable::new_constant(50.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Threshold",
            button_label: "+ Threshold",
            search_key: "threshold cutoff binary",
            id_prefix: "threshold",
            create_fn: |idx| Effect {
                id: format!("threshold_{}", idx),
                name: "Threshold".to_string(),
                effect_type: EffectType::Threshold {
                    threshold: Animatable::new_constant(128.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Linear Wipe",
            button_label: "+ Linear Wipe",
            search_key: "linear wipe transition reveal",
            id_prefix: "linwipe",
            create_fn: |idx| Effect {
                id: format!("linwipe_{}", idx),
                name: "Linear Wipe".to_string(),
                effect_type: EffectType::LinearWipe {
                    completion: Animatable::new_constant(0.0),
                    angle: Animatable::new_constant(0.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Simple Choker",
            button_label: "+ Simple Choker",
            search_key: "simple choker matte shrink grow",
            id_prefix: "choker",
            create_fn: |idx| Effect {
                id: format!("choker_{}", idx),
                name: "Simple Choker".to_string(),
                effect_type: EffectType::SimpleChoker {
                    choke_amount: Animatable::new_constant(0.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Chroma Key",
            button_label: "+ Chroma Key",
            search_key: "chroma key green screen keying",
            id_prefix: "chroma",
            create_fn: |idx| Effect {
                id: format!("chroma_{}", idx),
                name: "Chroma Key".to_string(),
                effect_type: EffectType::ChromaKey {
                    screen_color: Animatable::new_constant([0.0, 1.0, 0.0]),
                    screen_gain: Animatable::new_constant(1.0),
                    clip_black: Animatable::new_constant(0.0),
                    clip_white: Animatable::new_constant(1.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Spherize",
            button_label: "+ Spherize",
            search_key: "spherize sphere lens distortion cc sphere",
            id_prefix: "spherize",
            create_fn: |idx| Effect {
                id: format!("spherize_{}", idx),
                name: "Spherize".to_string(),
                effect_type: EffectType::Spherize {
                    radius: Animatable::new_constant(100.0),
                    refractive_index: Animatable::new_constant(1.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Turbulent Displace",
            button_label: "+ Turbulent Displace",
            search_key: "turbulent displace noise turbulence warp",
            id_prefix: "turbdisp",
            create_fn: |idx| Effect {
                id: format!("turbdisp_{}", idx),
                name: "Turbulent Displace".to_string(),
                effect_type: EffectType::TurbulentDisplace {
                    amount: Animatable::new_constant(25.0),
                    size: Animatable::new_constant(100.0),
                    evolution: Animatable::new_constant(0.0),
                    complexity: Animatable::new_constant(1.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Colorama",
            button_label: "+ Colorama",
            search_key: "colorama color cycle gradient rainbow",
            id_prefix: "colorama",
            create_fn: |idx| Effect {
                id: format!("colorama_{}", idx),
                name: "Colorama".to_string(),
                effect_type: EffectType::Colorama {
                    preset_index: Animatable::new_constant(0.0),
                    cycle_phase: Animatable::new_constant(0.0),
                },
                enabled: true,
            },
        },
        // ── New AE-standard effects ──

    ]
}
