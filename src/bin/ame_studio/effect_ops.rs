use super::*;

impl AfterEffectsStudio {
    pub(crate) fn add_effect(&mut self, kind: &str) {
        let Some(layer_id) = self.selected_layer_id.clone() else {
            return;
        };
        use kagari_vfx::core::timeline::{Animatable, EffectType};
        let (name, effect_type) = match kind {
            "blur" => (
                "Gaussian Blur",
                EffectType::GaussianBlur {
                    blur_radius: Animatable::new_constant(12.0),
                },
            ),
            "tint" => (
                "Color Tint",
                EffectType::ColorTint {
                    color: Animatable::new_constant([1.0, 0.45, 0.12, 1.0]),
                    intensity: Animatable::new_constant(50.0),
                },
            ),
            "shadow" => (
                "Drop Shadow",
                EffectType::DropShadow {
                    color: Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
                    opacity: Animatable::new_constant(65.0),
                    direction: Animatable::new_constant(135.0),
                    distance: Animatable::new_constant(12.0),
                    softness: Animatable::new_constant(8.0),
                },
            ),
            "glow" => (
                "Glow",
                EffectType::Glow {
                    threshold: Animatable::new_constant(65.0),
                    radius: Animatable::new_constant(12.0),
                    intensity: Animatable::new_constant(60.0),
                    color: Animatable::new_constant([1.0, 0.52, 0.2, 1.0]),
                },
            ),
            "vignette" => (
                "Vignette",
                EffectType::Vignette {
                    intensity: Animatable::new_constant(40.0),
                    roundness: Animatable::new_constant(1.0),
                    feather: Animatable::new_constant(70.0),
                    color: Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
                },
            ),
            "hue_saturation" => (
                "Hue / Saturation",
                EffectType::HueSaturation {
                    hue_shift: Animatable::new_constant(0.0),
                    saturation: Animatable::new_constant(0.0),
                    lightness: Animatable::new_constant(0.0),
                },
            ),
            "levels" => (
                "Levels",
                EffectType::Levels {
                    input_black: Animatable::new_constant(0.0),
                    input_white: Animatable::new_constant(1.0),
                    gamma: Animatable::new_constant(1.0),
                    output_black: Animatable::new_constant(0.0),
                    output_white: Animatable::new_constant(1.0),
                },
            ),
            "motion_blur" => (
                "Motion Blur",
                EffectType::MotionBlur {
                    shutter_angle: Animatable::new_constant(180.0),
                    samples: 8,
                },
            ),
            "film_grain" => (
                "Film Grain",
                EffectType::FilmGrain {
                    intensity: Animatable::new_constant(20.0),
                    grain_size: 1.0,
                    color_film: false,
                },
            ),
            "posterize" => (
                "Posterize",
                EffectType::Posterize {
                    levels: Animatable::new_constant(6.0),
                },
            ),
            "directional_blur" => (
                "Directional Blur",
                EffectType::DirectionalBlur {
                    angle: Animatable::new_constant(0.0),
                    length: Animatable::new_constant(12.0),
                },
            ),
            "radial_blur" => (
                "Radial Blur",
                EffectType::RadialBlur {
                    amount: Animatable::new_constant(0.2),
                },
            ),
            "sharpen" => (
                "Sharpen",
                EffectType::Sharpen {
                    amount: Animatable::new_constant(0.5),
                },
            ),
            "twirl" => (
                "Twirl",
                EffectType::Twirl {
                    angle: Animatable::new_constant(90.0),
                    radius: Animatable::new_constant(0.5),
                },
            ),
            "bulge" => (
                "Bulge",
                EffectType::Bulge {
                    amount: Animatable::new_constant(0.5),
                    radius: Animatable::new_constant(0.5),
                },
            ),
            "invert" => (
                "Invert",
                EffectType::Invert {
                    invert_alpha: false,
                },
            ),
            "offset" => (
                "Offset",
                EffectType::Offset {
                    shift_x: Animatable::new_constant(0.0),
                    shift_y: Animatable::new_constant(0.0),
                },
            ),
            "threshold" => (
                "Threshold",
                EffectType::Threshold {
                    threshold: Animatable::new_constant(0.5),
                },
            ),
            "linear_wipe" => (
                "Linear Wipe",
                EffectType::LinearWipe {
                    completion: Animatable::new_constant(0.0),
                    angle: Animatable::new_constant(0.0),
                },
            ),
            "spherize" => (
                "Spherize",
                EffectType::Spherize {
                    radius: Animatable::new_constant(0.5),
                    refractive_index: Animatable::new_constant(1.0),
                },
            ),
            "vibrance" => (
                "Vibrance",
                EffectType::Vibrance {
                    amount: Animatable::new_constant(25.0),
                },
            ),
            "white_balance" => (
                "White Balance",
                EffectType::WhiteBalance {
                    temperature: Animatable::new_constant(0.0),
                    tint: Animatable::new_constant(0.0),
                },
            ),
            "chromatic_aberration" => (
                "色収差",
                EffectType::ChromaticAberration {
                    shift_r: Animatable::new_constant(2.0),
                    shift_b: Animatable::new_constant(2.0),
                    edge_falloff: Animatable::new_constant(0.5),
                    iris_linked: false,
                },
            ),
            "chroma_key" => (
                "クロマキー",
                EffectType::ChromaKey {
                    screen_color: Animatable::new_constant([0.0, 1.0, 0.0]),
                    screen_gain: Animatable::new_constant(1.0),
                    clip_black: Animatable::new_constant(0.1),
                    clip_white: Animatable::new_constant(0.9),
                },
            ),
            "turbulent_displace" => (
                "タービュレントディスプレイス",
                EffectType::TurbulentDisplace {
                    amount: Animatable::new_constant(10.0),
                    size: Animatable::new_constant(40.0),
                    evolution: Animatable::new_constant(0.0),
                    complexity: Animatable::new_constant(2.0),
                },
            ),
            "cc_lens" => (
                "CC Lens",
                EffectType::CcLens {
                    convergence: Animatable::new_constant(2.0),
                    zoom: Animatable::new_constant(1.0),
                },
            ),
            "polar_coordinates" => (
                "極座標",
                EffectType::PolarCoordinates {
                    to_polar: false,
                    interpolation: Animatable::new_constant(0.0),
                },
            ),
            "wave_warp" => (
                "波形ワープ",
                EffectType::WaveWarp {
                    wave_height: Animatable::new_constant(20.0),
                    wave_width: Animatable::new_constant(50.0),
                    speed: Animatable::new_constant(1.0),
                    direction_deg: Animatable::new_constant(0.0),
                    wave_type: 0,
                    pinning: 0,
                },
            ),
            "vortex" => (
                "渦",
                EffectType::Vortex {
                    radius: Animatable::new_constant(0.5),
                    angle_deg: Animatable::new_constant(90.0),
                },
            ),
            "fisheye" => (
                "魚眼レンズ",
                EffectType::Fisheye {
                    strength: Animatable::new_constant(0.5),
                },
            ),
            "lens_correction" => (
                "レンズ補正",
                EffectType::LensCorrection {
                    k1: Animatable::new_constant(0.0),
                    k2: Animatable::new_constant(0.0),
                },
            ),
            "glitch_displacement" => (
                "グリッチディスプレイス",
                EffectType::GlitchDisplacement {
                    seed: Animatable::new_constant(0.0),
                    amount: Animatable::new_constant(1.0),
                },
            ),
            "tiler" => (
                "タイル",
                EffectType::Tiler {
                    scale_percent: Animatable::new_constant(100.0),
                    mirror: false,
                },
            ),
            "hsl_adjust" => (
                "HSL 調整",
                EffectType::HslAdjust {
                    hue_deg: Animatable::new_constant(0.0),
                    saturation: Animatable::new_constant(0.0),
                    lightness: Animatable::new_constant(0.0),
                },
            ),
            "crt_scanlines" => (
                "CRT スキャンライン",
                EffectType::CrtScanlines {
                    line_spacing: Animatable::new_constant(3.0),
                    intensity: Animatable::new_constant(20.0),
                },
            ),
            "shift_channels" => (
                "チャンネルシフト",
                EffectType::ShiftChannels {
                    take_red: Animatable::new_constant(0.0),
                    take_green: Animatable::new_constant(1.0),
                    take_blue: Animatable::new_constant(2.0),
                    take_alpha: Animatable::new_constant(3.0),
                },
            ),
            "minimax" => (
                "ミニマックス",
                EffectType::Minimax {
                    operation: Animatable::new_constant(0.0),
                    radius: Animatable::new_constant(3.0),
                },
            ),
            "fractal_noise" => (
                "フラクタルノイズ",
                EffectType::FractalNoise {
                    fractal_type: Animatable::new_constant(0.0),
                    contrast: Animatable::new_constant(90.0),
                    brightness: Animatable::new_constant(0.0),
                    complexity: Animatable::new_constant(2.0),
                    evolution: Animatable::new_constant(0.0),
                },
            ),
            "curves" => (
                "トーンカーブ",
                EffectType::Curves {
                    channel: Animatable::new_constant(0.0),
                },
            ),
            "compound_blur" => (
                "コンパウンドブラー",
                EffectType::CompoundBlur {
                    source_layer: Animatable::new_constant(0.0),
                    max_blur: Animatable::new_constant(20.0),
                },
            ),
            "displacement_map" => (
                "ディスプレイスメントマップ",
                EffectType::DisplacementMap {
                    source_layer: Animatable::new_constant(0.0),
                    max_horizontal: Animatable::new_constant(10.0),
                    max_vertical: Animatable::new_constant(10.0),
                },
            ),
            "mosaic" => (
                "モザイク",
                EffectType::Mosaic {
                    block_w: Animatable::new_constant(16.0),
                    block_h: Animatable::new_constant(16.0),
                },
            ),
            "flicker" => (
                "フリッカー",
                EffectType::Flicker {
                    amount: Animatable::new_constant(15.0),
                    speed: Animatable::new_constant(8.0),
                    seed: Animatable::new_constant(0.0),
                },
            ),
            "directional_sharpen" => (
                "方向シャープ",
                EffectType::DirectionalSharpen {
                    angle_deg: Animatable::new_constant(0.0),
                    strength: Animatable::new_constant(40.0),
                },
            ),
            "emboss" => (
                "エンボス",
                EffectType::Emboss {
                    angle_deg: Animatable::new_constant(45.0),
                    depth: Animatable::new_constant(50.0),
                },
            ),
            "bevel_alpha" => (
                "アルファベベル",
                EffectType::BevelAlpha {
                    depth: Animatable::new_constant(20.0),
                    light_angle_deg: Animatable::new_constant(45.0),
                },
            ),
            "letterbox" => (
                "レターボックス",
                EffectType::Letterbox {
                    frac: Animatable::new_constant(0.12),
                },
            ),
            "luma_key_range" => (
                "輝度キー",
                EffectType::LumaKeyRange {
                    low_threshold: Animatable::new_constant(0.2),
                    high_threshold: Animatable::new_constant(0.8),
                    invert: false,
                },
            ),
            "tritone" => (
                "トライトーン",
                EffectType::Tritone {
                    shadow_color: Animatable::new_constant([0.05, 0.05, 0.12]),
                    mid_color: Animatable::new_constant([0.5, 0.5, 0.55]),
                    highlight_color: Animatable::new_constant([1.0, 0.95, 0.8]),
                },
            ),
            "gradient_map" => (
                "グラデーションマップ",
                EffectType::GradientMap {
                    low_color: Animatable::new_constant([0.1, 0.14, 0.2]),
                    mid_color: Animatable::new_constant([0.48, 0.54, 0.72]),
                    high_color: Animatable::new_constant([1.0, 0.95, 0.84]),
                },
            ),
            "light_sweep" => (
                "ライトスイープ",
                EffectType::LightSweep {
                    direction_deg: Animatable::new_constant(0.0),
                    center: Animatable::new_constant(0.5),
                    width: Animatable::new_constant(0.1),
                    sweep_intensity: Animatable::new_constant(50.0),
                    edge_intensity: Animatable::new_constant(15.0),
                },
            ),
            "radial_fast_blur" => (
                "ラジアル高速ぼかし",
                EffectType::RadialFastBlur {
                    amount: Animatable::new_constant(20.0),
                    samples: 3,
                },
            ),
            "bend_it" => (
                "曲げ",
                EffectType::BendIt {
                    top_offset: Animatable::new_constant(0.0),
                    bottom_offset: Animatable::new_constant(0.0),
                },
            ),
            "venetian_blinds" => (
                "ベネチアンブラインド",
                EffectType::VenetianBlinds {
                    completion: Animatable::new_constant(0.0),
                    width: Animatable::new_constant(30.0),
                },
            ),
            "glow_pro" => (
                "Glow Pro",
                EffectType::GlowPro {
                    threshold: Animatable::new_constant(40.0),
                    radius: Animatable::new_constant(24.0),
                    intensity: Animatable::new_constant(80.0),
                },
            ),
            "sobel_edges" => ("Sobel Edges", EffectType::SobelEdges { invert: false }),
            "find_edges" => ("Find Edges", EffectType::FindEdges { invert: false }),
            "median_filter" => (
                "メディアンフィルター",
                EffectType::MedianFilter {
                    radius: Animatable::new_constant(2.0),
                },
            ),
            _ => return,
        };
        let id = format!(
            "ameui_effect_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default()
        );
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == layer_id && !layer.locked)
        else {
            return;
        };
        layer.effects.push(kagari_vfx::core::timeline::Effect {
            id: id.clone(),
            name: name.to_owned(),
            effect_type,
            enabled: true,
        });
        self.selected_effect_id = Some(id);
        self.mark_project_changed();
    }
}
