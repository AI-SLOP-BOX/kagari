use crate::core::property::Animatable;
use crate::core::timeline::{Effect, EffectType};
use super::EffectPreset;

pub(crate) fn get_presets_part3() -> Vec<EffectPreset> {
    vec![
        EffectPreset {
            name: "Mosaic",
            button_label: "+ Mosaic",
            search_key: "mosaic pixelate block censor blur squares",
            id_prefix: "mosaic",
            create_fn: |idx| Effect {
                id: format!("mosaic_{}", idx),
                name: "Mosaic".to_string(),
                effect_type: EffectType::Mosaic {
                    block_w: Animatable::new_constant(10.0),
                    block_h: Animatable::new_constant(10.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Tilt Shift",
            button_label: "+ Tilt Shift",
            search_key: "tilt shift miniature focus depth of field diorama",
            id_prefix: "tiltsh",
            create_fn: |idx| Effect {
                id: format!("tiltsh_{}", idx),
                name: "Tilt Shift".to_string(),
                effect_type: EffectType::TiltShift {
                    focus_y: Animatable::new_constant(0.5),
                    focus_height: Animatable::new_constant(0.3),
                    max_blur: Animatable::new_constant(6.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Emboss",
            button_label: "+ Emboss",
            search_key: "emboss relief 3d surface engrave",
            id_prefix: "emboss",
            create_fn: |idx| Effect {
                id: format!("emboss_{}", idx),
                name: "Emboss".to_string(),
                effect_type: EffectType::Emboss {
                    angle_deg: Animatable::new_constant(45.0),
                    depth: Animatable::new_constant(1.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Star Field",
            button_label: "+ Star Field",
            search_key: "star field space stars parallax night sky generate",
            id_prefix: "stars",
            create_fn: |idx| Effect {
                id: format!("stars_{}", idx),
                name: "Star Field".to_string(),
                effect_type: EffectType::StarField {
                    num_stars: Animatable::new_constant(150.0),
                    depth_speed: Animatable::new_constant(1.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Lightning",
            button_label: "+ Lightning",
            search_key: "lightning bolt electric storm arc thunder",
            id_prefix: "bolt",
            create_fn: |idx| Effect {
                id: format!("bolt_{}", idx),
                name: "Lightning".to_string(),
                effect_type: EffectType::LightningArc {
                    start_x: Animatable::new_constant(0.2),
                    start_y: Animatable::new_constant(0.0),
                    end_x: Animatable::new_constant(0.7),
                    end_y: Animatable::new_constant(1.0),
                    seed: Animatable::new_constant(3.0),
                    glow: Animatable::new_constant(1.5),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Laser Beam",
            button_label: "+ Laser Beam",
            search_key: "laser beam ray energy projectile blaster shoot glow core",
            id_prefix: "laser",
            create_fn: |idx| Effect {
                id: format!("laser_{}", idx),
                name: "Laser Beam".to_string(),
                effect_type: EffectType::LaserBeam {
                    start_x: Animatable::new_constant(0.1),
                    start_y: Animatable::new_constant(0.5),
                    end_x: Animatable::new_constant(0.9),
                    end_y: Animatable::new_constant(0.5),
                    progress: Animatable::new_constant(0.5),
                    length: Animatable::new_constant(40.0),
                    starting_thickness: Animatable::new_constant(12.0),
                    ending_thickness: Animatable::new_constant(4.0),
                    core_color: Animatable::new_constant([1.0, 1.0, 1.0, 1.0]),
                    glow_color: Animatable::new_constant([1.0, 0.2, 0.1, 0.8]),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Fire",
            button_label: "+ Fire",
            search_key: "fire flame burn cellular combustion heat",
            id_prefix: "firefx",
            create_fn: |idx| Effect {
                id: format!("firefx_{}", idx),
                name: "Fire".to_string(),
                effect_type: EffectType::FireAutomaton {
                    intensity: Animatable::new_constant(2.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Luma Key Range",
            button_label: "+ Luma Key Range",
            search_key: "luma key range luminance matte extract transparency",
            id_prefix: "lumakey",
            create_fn: |idx| Effect {
                id: format!("lumakey_{}", idx),
                name: "Luma Key Range".to_string(),
                effect_type: EffectType::LumaKeyRange {
                    low_threshold: Animatable::new_constant(40.0),
                    high_threshold: Animatable::new_constant(220.0),
                    invert: false,
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Halftone",
            button_label: "+ Halftone",
            search_key: "halftone dot screen print newspaper comic",
            id_prefix: "half",
            create_fn: |idx| Effect {
                id: format!("half_{}", idx),
                name: "Halftone".to_string(),
                effect_type: EffectType::Halftone {
                    cell_size: Animatable::new_constant(6.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Solarize",
            button_label: "+ Solarize",
            search_key: "solarize invert threshold sabattier negative",
            id_prefix: "sol",
            create_fn: |idx| Effect {
                id: format!("sol_{}", idx),
                name: "Solarize".to_string(),
                effect_type: EffectType::Solarize {
                    threshold: Animatable::new_constant(128.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Pixel Sort",
            button_label: "+ Pixel Sort",
            search_key: "pixel sort glitch columns datamosh aesthetic",
            id_prefix: "pixsort",
            create_fn: |idx| Effect {
                id: format!("pixsort_{}", idx),
                name: "Pixel Sort".to_string(),
                effect_type: EffectType::PixelSort {
                    threshold: Animatable::new_constant(140.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Pinch / Punch",
            button_label: "+ Pinch / Punch",
            search_key: "pinch punch polar distort squeeze bubble",
            id_prefix: "pinch",
            create_fn: |idx| Effect {
                id: format!("pinch_{}", idx),
                name: "Pinch / Punch".to_string(),
                effect_type: EffectType::PinchPunch {
                    radius: Animatable::new_constant(300.0),
                    amount: Animatable::new_constant(0.8),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Scanline Glitch",
            button_label: "+ Scanline Glitch",
            search_key: "scanline glitch jitter vhs signal noise rows",
            id_prefix: "sglitch",
            create_fn: |idx| Effect {
                id: format!("sglitch_{}", idx),
                name: "Scanline Glitch".to_string(),
                effect_type: EffectType::ScanlineGlitch {
                    jitter_amount: Animatable::new_constant(8.0),
                    seed: Animatable::new_constant(5.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Glass Edge Bevel",
            button_label: "+ Glass Edge Bevel",
            search_key: "glass edge bevel refraction specular frame border",
            id_prefix: "gbevel",
            create_fn: |idx| Effect {
                id: format!("gbevel_{}", idx),
                name: "Glass Edge Bevel".to_string(),
                effect_type: EffectType::GlassEdgeBevel {
                    bevel_size: Animatable::new_constant(12.0),
                    refraction: Animatable::new_constant(0.6),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Directional Sharpen",
            button_label: "+ Directional Sharpen",
            search_key: "directional sharpen angle motion enhance detail",
            id_prefix: "dsharp",
            create_fn: |idx| Effect {
                id: format!("dsharp_{}", idx),
                name: "Directional Sharpen".to_string(),
                effect_type: EffectType::DirectionalSharpen {
                    angle_deg: Animatable::new_constant(45.0),
                    strength: Animatable::new_constant(1.5),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Refraction Lens",
            button_label: "+ Refraction Lens",
            search_key: "refraction lens glass ball sphere ior crystal ball",
            id_prefix: "refrac",
            create_fn: |idx| Effect {
                id: format!("refrac_{}", idx),
                name: "Refraction Lens".to_string(),
                effect_type: EffectType::RefractionLens {
                    radius: Animatable::new_constant(150.0),
                    ior: Animatable::new_constant(1.4),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Gradient Map",
            button_label: "+ Gradient Map",
            search_key: "gradient map shadow mid high ramp duotone tritone colorize",
            id_prefix: "gradmap",
            create_fn: |idx| Effect {
                id: format!("gradmap_{}", idx),
                name: "Gradient Map".to_string(),
                effect_type: EffectType::GradientMap {
                    low_color: Animatable::new_constant([0.1, 0.1, 0.3]),
                    mid_color: Animatable::new_constant([0.6, 0.3, 0.4]),
                    high_color: Animatable::new_constant([1.0, 0.9, 0.7]),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Light Leak",
            button_label: "+ Light Leak",
            search_key: "light leak flare warm cinematic vintage overlay glow",
            id_prefix: "leak",
            create_fn: |idx| Effect {
                id: format!("leak_{}", idx),
                name: "Light Leak".to_string(),
                effect_type: EffectType::LightLeak {
                    pos_x: Animatable::new_constant(0.85),
                    pos_y: Animatable::new_constant(0.15),
                    intensity: Animatable::new_constant(1.2),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Lens Flare (GPU)",
            button_label: "+ Lens Flare (GPU)",
            search_key:
                "lens flare optical anamorphic streak star rings light source gpu screen space",
            id_prefix: "flare",
            create_fn: |idx| Effect {
                id: format!("flare_{}", idx),
                name: "Lens Flare".to_string(),
                effect_type: EffectType::LensFlare {
                    enabled: Animatable::new_constant(1.0),
                    position_x: Animatable::new_constant(0.5),
                    position_y: Animatable::new_constant(0.35),
                    intensity: Animatable::new_constant(1.0),
                    threshold: Animatable::new_constant(0.8),
                    color: Animatable::new_constant([1.0, 0.95, 0.9, 1.0]),
                    link_to_light: None,
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Bevel Alpha 3D",
            button_label: "+ Bevel Alpha 3D",
            search_key: "bevel alpha 3d inner contour highlight depth emboss edge",
            id_prefix: "balpha",
            create_fn: |idx| Effect {
                id: format!("balpha_{}", idx),
                name: "Bevel Alpha 3D".to_string(),
                effect_type: EffectType::BevelAlpha {
                    depth: Animatable::new_constant(6.0),
                    light_angle_deg: Animatable::new_constant(135.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Cross Hatch",
            button_label: "+ Cross Hatch",
            search_key: "cross hatch ink sketch drawing pen lines comic",
            id_prefix: "xhatch",
            create_fn: |idx| Effect {
                id: format!("xhatch_{}", idx),
                name: "Cross Hatch".to_string(),
                effect_type: EffectType::CrossHatch {
                    line_gap: Animatable::new_constant(8.0),
                    threshold: Animatable::new_constant(140.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "CMYK Halftone",
            button_label: "+ CMYK Halftone",
            search_key: "cmyk halftone print newspaper dots offset press",
            id_prefix: "cmhk",
            create_fn: |idx| Effect {
                id: format!("cmhk_{}", idx),
                name: "CMYK Halftone".to_string(),
                effect_type: EffectType::CmykHalftone {
                    dot_size: Animatable::new_constant(6.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Reflection Map",
            button_label: "+ Reflection Map",
            search_key: "reflection mirror water floor horizon fade",
            id_prefix: "reflmap",
            create_fn: |idx| Effect {
                id: format!("reflmap_{}", idx),
                name: "Reflection Map".to_string(),
                effect_type: EffectType::ReflectionMap {
                    reflect_y: Animatable::new_constant(200.0),
                    fade_dist: Animatable::new_constant(150.0),
                    opacity: Animatable::new_constant(0.6),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Perlin Flow Noise",
            button_label: "+ Perlin Flow",
            search_key: "perlin flow noise organic smoke fog generate procedural",
            id_prefix: "pflow",
            create_fn: |idx| Effect {
                id: format!("pflow_{}", idx),
                name: "Perlin Flow Noise".to_string(),
                effect_type: EffectType::PerlinFlow {
                    scale: Animatable::new_constant(4.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "FBM Turbulence",
            button_label: "+ FBM Turbulence",
            search_key: "fbm turbulence fractal brownian octaves displacement clouds",
            id_prefix: "fbmt",
            create_fn: |idx| Effect {
                id: format!("fbmt_{}", idx),
                name: "FBM Turbulence".to_string(),
                effect_type: EffectType::FbmTurbulence {
                    octaves: Animatable::new_constant(4.0),
                    amplitude: Animatable::new_constant(80.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Bass & Treble",
            button_label: "+ Bass & Treble",
            search_key: "bass treble eq equalizer bass boost treble cut crossover",
            id_prefix: "btreble",
            create_fn: |idx| Effect {
                id: format!("btreble_{}", idx),
                name: "Bass & Treble".to_string(),
                effect_type: EffectType::BassTreble {
                    bass_gain: Animatable::new_constant(0.0),
                    treble_gain: Animatable::new_constant(0.0),
                    crossover_freq: Animatable::new_constant(300.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Flanger",
            button_label: "+ Flanger",
            search_key: "flanger delay modulation lfo sweeping comb",
            id_prefix: "flanger",
            create_fn: |idx| Effect {
                id: format!("flanger_{}", idx),
                name: "Flanger".to_string(),
                effect_type: EffectType::Flanger {
                    max_delay_ms: Animatable::new_constant(5.0),
                    lfo_rate: Animatable::new_constant(0.5),
                    feedback: Animatable::new_constant(0.5),
                    wet_dry: Animatable::new_constant(0.5),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Chorus",
            button_label: "+ Chorus",
            search_key: "chorus detune multi-voice doubling modulation",
            id_prefix: "chorus",
            create_fn: |idx| Effect {
                id: format!("chorus_{}", idx),
                name: "Chorus".to_string(),
                effect_type: EffectType::Chorus {
                    delay_ms: Animatable::new_constant(15.0),
                    depth_ms: Animatable::new_constant(5.0),
                    rate_hz: Animatable::new_constant(1.0),
                    voices: Animatable::new_constant(3.0),
                    feedback: Animatable::new_constant(0.3),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Parametric EQ",
            button_label: "+ Parametric EQ",
            search_key: "parametric eq equalizer bell filter frequency resonance q",
            id_prefix: "peq",
            create_fn: |idx| Effect {
                id: format!("peq_{}", idx),
                name: "Parametric EQ".to_string(),
                effect_type: EffectType::ParametricEQ {
                    freq_hz: Animatable::new_constant(1000.0),
                    gain_db: Animatable::new_constant(0.0),
                    q_factor: Animatable::new_constant(1.0),
                },
                enabled: true,
            },
        },
    ]
}
