use super::*;

pub(crate) fn rgba_to_hex(color: &[f32; 4]) -> String {
    format!(
        "#{:02X}{:02X}{:02X}",
        (color[0].clamp(0.0, 1.0) * 255.0).round() as u8,
        (color[1].clamp(0.0, 1.0) * 255.0).round() as u8,
        (color[2].clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

pub(crate) fn effect_type_label(effect: &kagari_vfx::core::timeline::EffectType) -> &'static str {
    use kagari_vfx::core::timeline::EffectType;
    match effect {
        EffectType::GaussianBlur { .. } => "ぼかし",
        EffectType::ColorTint { .. } => "色調",
        EffectType::DropShadow { .. } => "影",
        EffectType::Vignette { .. } => "ビネット",
        EffectType::Levels { .. } => "Levels",
        EffectType::HueSaturation { .. } => "HSL",
        EffectType::Glow { .. } => "Glow",
        EffectType::MotionBlur { .. } => "モーションブラー",
        EffectType::FilmGrain { .. } => "フィルムグレイン",
        EffectType::Posterize { .. } => "ポスタライズ",
        EffectType::DirectionalBlur { .. } => "方向ぼかし",
        EffectType::RadialBlur { .. } => "放射状ぼかし",
        EffectType::Sharpen { .. } => "シャープ",
        EffectType::Twirl { .. } => "渦巻き",
        EffectType::Bulge { .. } => "膨張・収縮",
        EffectType::Invert { .. } => "反転",
        EffectType::Offset { .. } => "オフセット",
        EffectType::Threshold { .. } => "しきい値",
        EffectType::LinearWipe { .. } => "リニアワイプ",
        EffectType::Spherize { .. } => "球面化",
        EffectType::Vibrance { .. } => "自然な彩度",
        EffectType::WhiteBalance { .. } => "ホワイトバランス",
        EffectType::ChromaticAberration { .. } => "色収差",
        EffectType::ChromaKey { .. } => "クロマキー",
        EffectType::TurbulentDisplace { .. } => "タービュレントディスプレイス",
        EffectType::CcLens { .. } => "CC Lens",
        EffectType::PolarCoordinates { .. } => "極座標",
        EffectType::WaveWarp { .. } => "波形ワープ",
        EffectType::Vortex { .. } => "渦",
        EffectType::Fisheye { .. } => "魚眼レンズ",
        EffectType::LensCorrection { .. } => "レンズ補正",
        EffectType::GlitchDisplacement { .. } => "グリッチディスプレイス",
        EffectType::Tiler { .. } => "タイル",
        EffectType::HslAdjust { .. } => "HSL 調整",
        EffectType::CrtScanlines { .. } => "CRT スキャンライン",
        EffectType::ShiftChannels { .. } => "チャンネルシフト",
        EffectType::Minimax { .. } => "ミニマックス",
        EffectType::FractalNoise { .. } => "フラクタルノイズ",
        EffectType::Curves { .. } => "トーンカーブ",
        EffectType::CompoundBlur { .. } => "コンパウンドブラー",
        EffectType::DisplacementMap { .. } => "ディスプレイスメントマップ",
        EffectType::Mosaic { .. } => "モザイク",
        EffectType::Flicker { .. } => "フリッカー",
        EffectType::DirectionalSharpen { .. } => "方向シャープ",
        EffectType::Emboss { .. } => "エンボス",
        EffectType::BevelAlpha { .. } => "アルファベベル",
        EffectType::Letterbox { .. } => "レターボックス",
        EffectType::LumaKeyRange { .. } => "輝度キー",
        EffectType::Tritone { .. } => "トライトーン",
        EffectType::GradientMap { .. } => "グラデーションマップ",
        EffectType::LightSweep { .. } => "ライトスイープ",
        EffectType::RadialFastBlur { .. } => "ラジアル高速ぼかし",
        EffectType::BendIt { .. } => "曲げ",
        EffectType::VenetianBlinds { .. } => "ベネチアンブラインド",
        EffectType::GlowPro { .. } => "Glow Pro",
        EffectType::SobelEdges { .. } => "Sobel Edges",
        EffectType::FindEdges { .. } => "Find Edges",
        EffectType::MedianFilter { .. } => "メディアンフィルター",
        _ => "その他",
    }
}

pub(crate) fn effect_parameter_specs(
    effect: &kagari_vfx::core::timeline::EffectType,
    frame: u32,
) -> Vec<(&'static str, &'static str, f32, &'static str)> {
    use kagari_vfx::core::timeline::EffectType;
    let mut specs = match effect {
        EffectType::GaussianBlur { blur_radius } => {
            vec![("blur_radius", "半径", blur_radius.evaluate(frame), "px")]
        }
        EffectType::ColorTint { intensity, .. } => {
            vec![("intensity", "強度", intensity.evaluate(frame), "%")]
        }
        EffectType::DropShadow {
            opacity,
            direction,
            distance,
            softness,
            ..
        } => vec![
            ("opacity", "不透明度", opacity.evaluate(frame), "%"),
            ("direction", "角度", direction.evaluate(frame), "°"),
            ("distance", "距離", distance.evaluate(frame), "px"),
            ("softness", "ぼかし", softness.evaluate(frame), "px"),
        ],
        EffectType::Vignette {
            intensity,
            roundness,
            feather,
            ..
        } => vec![
            ("intensity", "強度", intensity.evaluate(frame), "%"),
            ("roundness", "丸み", roundness.evaluate(frame), ""),
            ("feather", "ぼかし", feather.evaluate(frame), "%"),
        ],
        EffectType::Levels {
            input_black,
            input_white,
            gamma,
            output_black,
            output_white,
        } => vec![
            ("input_black", "入力黒", input_black.evaluate(frame), ""),
            ("input_white", "入力白", input_white.evaluate(frame), ""),
            ("gamma", "ガンマ", gamma.evaluate(frame), ""),
            ("output_black", "出力黒", output_black.evaluate(frame), ""),
            ("output_white", "出力白", output_white.evaluate(frame), ""),
        ],
        EffectType::HueSaturation {
            hue_shift,
            saturation,
            lightness,
        } => vec![
            ("hue_shift", "色相", hue_shift.evaluate(frame), "°"),
            ("saturation", "彩度", saturation.evaluate(frame), "%"),
            ("lightness", "明度", lightness.evaluate(frame), "%"),
        ],
        EffectType::Glow {
            threshold,
            radius,
            intensity,
            ..
        } => vec![
            ("threshold", "しきい値", threshold.evaluate(frame), "%"),
            ("radius", "半径", radius.evaluate(frame), "px"),
            ("intensity", "強度", intensity.evaluate(frame), "%"),
        ],
        EffectType::MotionBlur { shutter_angle, .. } => vec![(
            "shutter_angle",
            "シャッター角",
            shutter_angle.evaluate(frame),
            "°",
        )],
        EffectType::FilmGrain { intensity, .. } => {
            vec![("intensity", "強度", intensity.evaluate(frame), "%")]
        }
        EffectType::Posterize { levels } => {
            vec![("levels", "階調数", levels.evaluate(frame), "段")]
        }
        EffectType::DirectionalBlur { angle, length } => vec![
            ("angle", "角度", angle.evaluate(frame), "°"),
            ("length", "長さ", length.evaluate(frame), "px"),
        ],
        EffectType::RadialBlur { amount } => {
            vec![("amount", "強度", amount.evaluate(frame), "×")]
        }
        EffectType::Sharpen { amount } => {
            vec![("amount", "量", amount.evaluate(frame), "×")]
        }
        EffectType::ChromaticAberration {
            shift_r,
            shift_b,
            edge_falloff,
            ..
        } => vec![
            ("shift_r", "赤シフト", shift_r.evaluate(frame), "px"),
            ("shift_b", "青シフト", shift_b.evaluate(frame), "px"),
            ("edge_falloff", "端の減衰", edge_falloff.evaluate(frame), ""),
        ],
        EffectType::ChromaKey {
            screen_gain,
            clip_black,
            clip_white,
            ..
        } => vec![
            ("screen_gain", "ゲイン", screen_gain.evaluate(frame), ""),
            ("clip_black", "黒クリップ", clip_black.evaluate(frame), ""),
            ("clip_white", "白クリップ", clip_white.evaluate(frame), ""),
        ],
        EffectType::TurbulentDisplace {
            amount,
            size,
            evolution,
            complexity,
        } => vec![
            ("amount", "量", amount.evaluate(frame), "px"),
            ("size", "サイズ", size.evaluate(frame), "px"),
            ("evolution", "進化", evolution.evaluate(frame), ""),
            ("complexity", "複雑さ", complexity.evaluate(frame), ""),
        ],
        EffectType::CcLens { convergence, zoom } => vec![
            ("convergence", "収束", convergence.evaluate(frame), ""),
            ("zoom", "ズーム", zoom.evaluate(frame), ""),
        ],
        EffectType::PolarCoordinates { interpolation, .. } => {
            vec![("interpolation", "補間", interpolation.evaluate(frame), "")]
        }
        EffectType::WaveWarp {
            wave_height,
            wave_width,
            speed,
            direction_deg,
            ..
        } => vec![
            ("wave_height", "波の高さ", wave_height.evaluate(frame), "px"),
            ("wave_width", "波の幅", wave_width.evaluate(frame), "px"),
            ("speed", "速度", speed.evaluate(frame), ""),
            ("direction_deg", "方向", direction_deg.evaluate(frame), "°"),
        ],
        EffectType::Vortex { radius, angle_deg } => vec![
            ("radius", "半径", radius.evaluate(frame), ""),
            ("angle_deg", "角度", angle_deg.evaluate(frame), "°"),
        ],
        EffectType::Fisheye { strength } => {
            vec![("strength", "強度", strength.evaluate(frame), "")]
        }
        EffectType::LensCorrection { k1, k2 } => vec![
            ("k1", "K1", k1.evaluate(frame), ""),
            ("k2", "K2", k2.evaluate(frame), ""),
        ],
        EffectType::GlitchDisplacement { seed, amount } => vec![
            ("seed", "シード", seed.evaluate(frame), ""),
            ("amount", "量", amount.evaluate(frame), ""),
        ],
        EffectType::Tiler { scale_percent, .. } => {
            vec![(
                "scale_percent",
                "スケール",
                scale_percent.evaluate(frame),
                "%",
            )]
        }
        EffectType::HslAdjust {
            hue_deg,
            saturation,
            lightness,
        } => vec![
            ("hue_deg", "色相", hue_deg.evaluate(frame), "°"),
            ("saturation", "彩度", saturation.evaluate(frame), "%"),
            ("lightness", "明度", lightness.evaluate(frame), "%"),
        ],
        EffectType::CrtScanlines {
            line_spacing,
            intensity,
        } => vec![
            ("line_spacing", "線間隔", line_spacing.evaluate(frame), "px"),
            ("intensity", "強度", intensity.evaluate(frame), "%"),
        ],
        EffectType::ShiftChannels {
            take_red,
            take_green,
            take_blue,
            take_alpha,
        } => vec![
            ("take_red", "赤", take_red.evaluate(frame), ""),
            ("take_green", "緑", take_green.evaluate(frame), ""),
            ("take_blue", "青", take_blue.evaluate(frame), ""),
            ("take_alpha", "アルファ", take_alpha.evaluate(frame), ""),
        ],
        EffectType::Minimax { operation, radius } => vec![
            ("operation", "演算", operation.evaluate(frame), ""),
            ("radius", "半径", radius.evaluate(frame), "px"),
        ],
        EffectType::FractalNoise {
            contrast,
            brightness,
            complexity,
            evolution,
            ..
        } => vec![
            ("contrast", "コントラスト", contrast.evaluate(frame), "%"),
            ("brightness", "明るさ", brightness.evaluate(frame), ""),
            ("complexity", "複雑さ", complexity.evaluate(frame), ""),
            ("evolution", "進化", evolution.evaluate(frame), ""),
        ],
        EffectType::Curves { channel } => {
            vec![("channel", "チャンネル", channel.evaluate(frame), "")]
        }
        EffectType::CompoundBlur { max_blur, .. } => {
            vec![("max_blur", "最大ぼかし", max_blur.evaluate(frame), "px")]
        }
        EffectType::DisplacementMap {
            max_horizontal,
            max_vertical,
            ..
        } => vec![
            (
                "max_horizontal",
                "水平量",
                max_horizontal.evaluate(frame),
                "px",
            ),
            ("max_vertical", "垂直量", max_vertical.evaluate(frame), "px"),
        ],
        EffectType::Mosaic { block_w, block_h } => vec![
            ("block_w", "幅", block_w.evaluate(frame), "px"),
            ("block_h", "高さ", block_h.evaluate(frame), "px"),
        ],
        EffectType::Flicker {
            amount,
            speed,
            seed,
        } => vec![
            ("amount", "量", amount.evaluate(frame), "%"),
            ("speed", "速度", speed.evaluate(frame), ""),
            ("seed", "シード", seed.evaluate(frame), ""),
        ],
        EffectType::DirectionalSharpen {
            angle_deg,
            strength,
        } => vec![
            ("angle_deg", "角度", angle_deg.evaluate(frame), "°"),
            ("strength", "強度", strength.evaluate(frame), "%"),
        ],
        EffectType::Emboss { angle_deg, depth } => vec![
            ("angle_deg", "角度", angle_deg.evaluate(frame), "°"),
            ("depth", "深度", depth.evaluate(frame), "%"),
        ],
        EffectType::BevelAlpha {
            depth,
            light_angle_deg,
        } => vec![
            ("depth", "深度", depth.evaluate(frame), "%"),
            (
                "light_angle_deg",
                "ライト角度",
                light_angle_deg.evaluate(frame),
                "°",
            ),
        ],
        EffectType::Letterbox { frac } => vec![("frac", "比率", frac.evaluate(frame), "")],
        EffectType::LumaKeyRange {
            low_threshold,
            high_threshold,
            ..
        } => vec![
            (
                "low_threshold",
                "低しきい値",
                low_threshold.evaluate(frame),
                "",
            ),
            (
                "high_threshold",
                "高しきい値",
                high_threshold.evaluate(frame),
                "",
            ),
        ],
        EffectType::LightSweep {
            direction_deg,
            center,
            width,
            sweep_intensity,
            edge_intensity,
        } => vec![
            ("direction_deg", "方向", direction_deg.evaluate(frame), "°"),
            ("center", "中心", center.evaluate(frame), ""),
            ("width", "幅", width.evaluate(frame), ""),
            (
                "sweep_intensity",
                "スイープ強度",
                sweep_intensity.evaluate(frame),
                "%",
            ),
            (
                "edge_intensity",
                "エッジ強度",
                edge_intensity.evaluate(frame),
                "%",
            ),
        ],
        EffectType::RadialFastBlur { amount, .. } => {
            vec![("amount", "量", amount.evaluate(frame), "%")]
        }
        EffectType::BendIt {
            top_offset,
            bottom_offset,
        } => vec![
            (
                "top_offset",
                "上オフセット",
                top_offset.evaluate(frame),
                "px",
            ),
            (
                "bottom_offset",
                "下オフセット",
                bottom_offset.evaluate(frame),
                "px",
            ),
        ],
        EffectType::VenetianBlinds { completion, width } => vec![
            ("completion", "完了率", completion.evaluate(frame), "%"),
            ("width", "幅", width.evaluate(frame), "px"),
        ],
        EffectType::GlowPro {
            threshold,
            radius,
            intensity,
        } => vec![
            ("threshold", "しきい値", threshold.evaluate(frame), "%"),
            ("radius", "半径", radius.evaluate(frame), "px"),
            ("intensity", "強度", intensity.evaluate(frame), "%"),
        ],
        EffectType::MedianFilter { radius } => {
            vec![("radius", "半径", radius.evaluate(frame), "px")]
        }
        _ => Vec::new(),
    };
    use kagari_vfx::core::effect_params::ParamRefRef;
    let known = specs
        .iter()
        .map(|(id, _, _, _)| *id)
        .collect::<std::collections::HashSet<_>>();
    for (id, parameter) in effect.animatable_params_ref() {
        if known.contains(id) {
            continue;
        }
        if let ParamRefRef::Scalar(track) = parameter {
            // Expose reflected scalar tracks for newly wired effects without
            // requiring a hand-written property row for every effect type.
            specs.push((id, id, track.evaluate(frame), ""));
        }
    }
    specs
}

pub(crate) fn scalar_effect_parameters(
    effect: &kagari_vfx::core::timeline::EffectType,
) -> Vec<(String, String)> {
    use kagari_vfx::core::effect_params::ParamRefRef;
    let labels = effect_parameter_specs(effect, 0)
        .into_iter()
        .map(|(id, label, _, _)| (id, label))
        .collect::<std::collections::HashMap<_, _>>();
    effect
        .animatable_params_ref()
        .into_iter()
        .filter_map(|(id, parameter)| {
            matches!(parameter, ParamRefRef::Scalar(_)).then(|| {
                (
                    id.to_owned(),
                    labels.get(id).copied().unwrap_or(id).to_owned(),
                )
            })
        })
        .collect()
}

pub(crate) fn effect_color_parameters(
    effect: &kagari_vfx::core::timeline::EffectType,
) -> Vec<(String, String)> {
    use kagari_vfx::core::effect_params::ParamRefRef;
    effect
        .animatable_params_ref()
        .into_iter()
        .filter_map(|(id, parameter)| {
            matches!(parameter, ParamRefRef::Vec4Color(_)).then(|| (id.to_owned(), id.to_owned()))
        })
        .collect()
}

pub(crate) fn graph_effect_parameter_label(
    layer: &kagari_vfx::core::timeline::Layer,
    effect_id: Option<&str>,
    parameter_id: Option<&str>,
) -> Option<String> {
    let effect = layer
        .effects
        .iter()
        .find(|effect| Some(effect.id.as_str()) == effect_id)?;
    scalar_effect_parameters(&effect.effect_type)
        .into_iter()
        .find(|(id, _)| Some(id.as_str()) == parameter_id)
        .map(|(_, label)| format!("{} · {}", effect.name, label))
}

pub(crate) fn scalar_effect_track<'a>(
    layer: &'a kagari_vfx::core::timeline::Layer,
    effect_id: Option<&str>,
    parameter_id: Option<&str>,
) -> Option<&'a kagari_vfx::core::timeline::Animatable<f32>> {
    let effect = layer
        .effects
        .iter()
        .find(|effect| Some(effect.id.as_str()) == effect_id)?;
    effect
        .effect_type
        .animatable_params_ref()
        .into_iter()
        .find_map(|(id, parameter)| {
            if Some(id) != parameter_id {
                return None;
            }
            match parameter {
                kagari_vfx::core::effect_params::ParamRefRef::Scalar(track) => Some(track),
                _ => None,
            }
        })
}

pub(crate) fn scalar_effect_track_mut<'a>(
    layer: &'a mut kagari_vfx::core::timeline::Layer,
    effect_id: Option<&str>,
    parameter_id: Option<&str>,
) -> Option<&'a mut kagari_vfx::core::timeline::Animatable<f32>> {
    let effect = layer
        .effects
        .iter_mut()
        .find(|effect| Some(effect.id.as_str()) == effect_id)?;
    effect
        .effect_type
        .animatable_params()
        .into_iter()
        .find_map(|(id, parameter)| {
            if Some(id) != parameter_id {
                return None;
            }
            match parameter {
                kagari_vfx::core::effect_params::ParamRef::Scalar(track) => Some(track),
                _ => None,
            }
        })
}

pub(crate) fn effect_color_track<'a>(
    layer: &'a kagari_vfx::core::timeline::Layer,
    effect_id: Option<&str>,
    parameter_id: Option<&str>,
) -> Option<&'a kagari_vfx::core::timeline::Animatable<[f32; 4]>> {
    let effect = layer
        .effects
        .iter()
        .find(|effect| Some(effect.id.as_str()) == effect_id)?;
    effect
        .effect_type
        .animatable_params_ref()
        .into_iter()
        .find_map(|(id, parameter)| {
            if Some(id) != parameter_id {
                return None;
            }
            match parameter {
                kagari_vfx::core::effect_params::ParamRefRef::Vec4Color(track) => Some(track),
                _ => None,
            }
        })
}

pub(crate) fn effect_color_track_mut<'a>(
    layer: &'a mut kagari_vfx::core::timeline::Layer,
    effect_id: Option<&str>,
    parameter_id: Option<&str>,
) -> Option<&'a mut kagari_vfx::core::timeline::Animatable<[f32; 4]>> {
    let effect = layer
        .effects
        .iter_mut()
        .find(|effect| Some(effect.id.as_str()) == effect_id)?;
    effect
        .effect_type
        .animatable_params()
        .into_iter()
        .find_map(|(id, parameter)| {
            if Some(id) != parameter_id {
                return None;
            }
            match parameter {
                kagari_vfx::core::effect_params::ParamRef::Vec4Color(track) => Some(track),
                _ => None,
            }
        })
}

pub(crate) fn effect_color_component(channel: GraphChannel) -> Option<usize> {
    match channel {
        GraphChannel::EffectColorRed => Some(0),
        GraphChannel::EffectColorGreen => Some(1),
        GraphChannel::EffectColorBlue => Some(2),
        GraphChannel::EffectColorAlpha => Some(3),
        _ => None,
    }
}

pub(crate) fn graph_color_component_name(channel: GraphChannel) -> &'static str {
    match channel {
        GraphChannel::EffectColorRed => "R",
        GraphChannel::EffectColorGreen => "G",
        GraphChannel::EffectColorBlue => "B",
        GraphChannel::EffectColorAlpha => "A",
        _ => "",
    }
}

pub(crate) fn update_color_component_graph_track(
    track: &mut kagari_vfx::core::timeline::Animatable<[f32; 4]>,
    component: usize,
    source_frame: u32,
    target_frame: u32,
    value: f32,
) -> bool {
    if track.keyframes().is_none() {
        return false;
    }
    if source_frame != target_frame && !track.move_keyframe(source_frame, target_frame) {
        return false;
    }
    let mut color = track.evaluate(target_frame);
    color[component] = value;
    track.set_value_at_frame(target_frame, color);
    true
}

pub(crate) fn shape_parameter_track(
    layer: &kagari_vfx::core::timeline::Layer,
    channel: GraphChannel,
) -> Option<&kagari_vfx::core::timeline::Animatable<f32>> {
    use kagari_vfx::core::timeline::{LayerType, ShapeType};
    let LayerType::Shape { shape_type, .. } = &layer.layer_type else {
        return None;
    };
    match (shape_type, channel) {
        (
            ShapeType::Rectangle { width, .. } | ShapeType::Ellipse { width, .. },
            GraphChannel::ShapeWidth,
        ) => Some(width),
        (
            ShapeType::Rectangle { height, .. } | ShapeType::Ellipse { height, .. },
            GraphChannel::ShapeHeight,
        ) => Some(height),
        (ShapeType::Rectangle { corner_radius, .. }, GraphChannel::ShapeCornerRadius) => {
            Some(corner_radius)
        }
        (ShapeType::Star { points, .. }, GraphChannel::ShapePoints) => Some(points),
        (ShapeType::Star { inner_radius, .. }, GraphChannel::ShapeInnerRadius) => {
            Some(inner_radius)
        }
        (ShapeType::Star { outer_radius, .. }, GraphChannel::ShapeOuterRadius) => {
            Some(outer_radius)
        }
        (ShapeType::Polygon { sides, .. }, GraphChannel::ShapeSides) => Some(sides),
        (ShapeType::Polygon { radius, .. }, GraphChannel::ShapeRadius) => Some(radius),
        _ => None,
    }
}

pub(crate) fn shape_parameter_track_mut(
    layer: &mut kagari_vfx::core::timeline::Layer,
    channel: GraphChannel,
) -> Option<&mut kagari_vfx::core::timeline::Animatable<f32>> {
    use kagari_vfx::core::timeline::{LayerType, ShapeType};
    let LayerType::Shape { shape_type, .. } = &mut layer.layer_type else {
        return None;
    };
    match (shape_type, channel) {
        (
            ShapeType::Rectangle { width, .. } | ShapeType::Ellipse { width, .. },
            GraphChannel::ShapeWidth,
        ) => Some(width),
        (
            ShapeType::Rectangle { height, .. } | ShapeType::Ellipse { height, .. },
            GraphChannel::ShapeHeight,
        ) => Some(height),
        (ShapeType::Rectangle { corner_radius, .. }, GraphChannel::ShapeCornerRadius) => {
            Some(corner_radius)
        }
        (ShapeType::Star { points, .. }, GraphChannel::ShapePoints) => Some(points),
        (ShapeType::Star { inner_radius, .. }, GraphChannel::ShapeInnerRadius) => {
            Some(inner_radius)
        }
        (ShapeType::Star { outer_radius, .. }, GraphChannel::ShapeOuterRadius) => {
            Some(outer_radius)
        }
        (ShapeType::Polygon { sides, .. }, GraphChannel::ShapeSides) => Some(sides),
        (ShapeType::Polygon { radius, .. }, GraphChannel::ShapeRadius) => Some(radius),
        _ => None,
    }
}

pub(crate) fn set_f32_if_changed(target: &mut f32, next: f32) -> bool {
    if !next.is_finite() || (*target - next).abs() <= f32::EPSILON {
        false
    } else {
        *target = next;
        true
    }
}

pub(crate) fn parse_hex_rgb(input: &str, alpha: f32) -> Option<[f32; 4]> {
    let value = input.trim().strip_prefix('#').unwrap_or(input.trim());
    if value.len() != 6 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let red = u8::from_str_radix(&value[0..2], 16).ok()?;
    let green = u8::from_str_radix(&value[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&value[4..6], 16).ok()?;
    Some([
        f32::from(red) / 255.0,
        f32::from(green) / 255.0,
        f32::from(blue) / 255.0,
        alpha.clamp(0.0, 1.0),
    ])
}

pub(crate) fn add_current_keyframe<T: kagari_vfx::core::property::Interpolate>(
    track: &mut kagari_vfx::core::timeline::Animatable<T>,
    frame: u32,
) {
    let value = track.evaluate(frame);
    track.set_keyframe(kagari_vfx::core::keyframe::Keyframe::new(
        frame,
        value,
        kagari_vfx::core::keyframe::InterpolationType::Linear,
    ));
}

pub(crate) fn set_animatable_at_frame<T: Clone>(
    track: &mut kagari_vfx::core::timeline::Animatable<T>,
    frame: u32,
    value: T,
) -> bool {
    if track.keyframes().is_some() {
        track.set_keyframe(kagari_vfx::core::keyframe::Keyframe::new(
            frame,
            value,
            kagari_vfx::core::keyframe::InterpolationType::Linear,
        ));
    } else {
        *track = kagari_vfx::core::timeline::Animatable::new_constant(value);
    }
    true
}

pub(crate) fn effect_parameter_range(
    effect: &kagari_vfx::core::timeline::EffectType,
    parameter_id: &str,
) -> (f32, f32) {
    use kagari_vfx::core::timeline::EffectType;
    match (effect, parameter_id) {
        (EffectType::GaussianBlur { .. }, "blur_radius") => (0.0, 200.0),
        (EffectType::ColorTint { .. }, "intensity") => (0.0, 100.0),
        (EffectType::DropShadow { .. }, "opacity") => (0.0, 100.0),
        (EffectType::DropShadow { .. }, "direction") => (-180.0, 180.0),
        (EffectType::DropShadow { .. }, "distance") => (0.0, 200.0),
        (EffectType::DropShadow { .. }, "softness") => (0.0, 100.0),
        (EffectType::Levels { .. }, "gamma") => (0.1, 4.0),
        (EffectType::Levels { .. }, _) => (0.0, 1.0),
        (EffectType::HueSaturation { .. }, "hue") => (-180.0, 180.0),
        (EffectType::HueSaturation { .. }, "saturation" | "lightness") => (-100.0, 100.0),
        (EffectType::FilmEmulation { .. }, "lift") => (-0.5, 0.5),
        (EffectType::FilmEmulation { .. }, "gamma" | "gain") => (0.2, 2.2),
        (EffectType::FilmEmulation { .. }, "hue_shift_deg") => (-45.0, 45.0),
        (EffectType::FilmEmulation { .. }, _) => (0.0, 2.0),
        (EffectType::Glow { .. }, "intensity") => (0.0, 5.0),
        (EffectType::Glow { .. }, "radius") => (0.0, 200.0),
        (EffectType::Glow { .. }, "threshold") => (0.0, 1.0),
        (EffectType::RadialBlur { .. }, "amount") => (0.0, 1.0),
        (EffectType::Sharpen { .. }, "amount") => (0.0, 5.0),
        (_, "opacity" | "intensity" | "amount") => (0.0, 100.0),
        (_, "angle" | "direction" | "hue" | "hue_shift_deg") => (-180.0, 180.0),
        (_, "gamma" | "gain") => (0.1, 4.0),
        (_, "lift") => (-0.5, 0.5),
        (_, "radius" | "blur_radius" | "distance" | "softness") => (0.0, 200.0),
        _ => (0.0, 100.0),
    }
}

pub(crate) fn normalize_effect_param(value: f32, min: f32, max: f32) -> f64 {
    let span = (max - min).max(1e-5);
    f64::from(((value - min) / span).clamp(0.0, 1.0))
}
