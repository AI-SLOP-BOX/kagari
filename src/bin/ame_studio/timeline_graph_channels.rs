#![allow(clippy::too_many_arguments)]
use super::*;

pub(crate) fn graph_channel_keys(
    layer: &kagari_vfx::core::timeline::Layer,
    camera: Option<&kagari_vfx::core::timeline::Camera3D>,
    light: Option<&kagari_vfx::core::timeline::Light3D>,
    mask_id: Option<&str>,
    effect_id: Option<&str>,
    effect_parameter_id: Option<&str>,
    channel: GraphChannel,
) -> Vec<(u32, f32)> {
    let keys = match channel {
        GraphChannel::PositionX => layer.transform.position.keyframes().map(|keys| {
            keys.iter()
                .map(|key| (key.frame, key.value[0]))
                .collect::<Vec<_>>()
        }),
        GraphChannel::PositionY => layer.transform.position.keyframes().map(|keys| {
            keys.iter()
                .map(|key| (key.frame, key.value[1]))
                .collect::<Vec<_>>()
        }),
        GraphChannel::AnchorX => layer
            .transform
            .anchor_point
            .keyframes()
            .map(|keys| keys.iter().map(|key| (key.frame, key.value[0])).collect()),
        GraphChannel::AnchorY => layer
            .transform
            .anchor_point
            .keyframes()
            .map(|keys| keys.iter().map(|key| (key.frame, key.value[1])).collect()),
        GraphChannel::ScaleX => layer.transform.scale.keyframes().map(|keys| {
            keys.iter()
                .map(|key| (key.frame, key.value[0]))
                .collect::<Vec<_>>()
        }),
        GraphChannel::ScaleY => layer.transform.scale.keyframes().map(|keys| {
            keys.iter()
                .map(|key| (key.frame, key.value[1]))
                .collect::<Vec<_>>()
        }),
        GraphChannel::Rotation => layer.transform.rotation.keyframes().map(|keys| {
            keys.iter()
                .map(|key| (key.frame, key.value))
                .collect::<Vec<_>>()
        }),
        GraphChannel::Opacity => layer.transform.opacity.keyframes().map(|keys| {
            keys.iter()
                .map(|key| (key.frame, key.value))
                .collect::<Vec<_>>()
        }),
        GraphChannel::Position3DX => layer
            .transform_3d
            .position
            .keyframes()
            .map(|keys| keys.iter().map(|key| (key.frame, key.value[0])).collect()),
        GraphChannel::Position3DY => layer
            .transform_3d
            .position
            .keyframes()
            .map(|keys| keys.iter().map(|key| (key.frame, key.value[1])).collect()),
        GraphChannel::Position3DZ => layer
            .transform_3d
            .position
            .keyframes()
            .map(|keys| keys.iter().map(|key| (key.frame, key.value[2])).collect()),
        GraphChannel::Rotation3DX => layer
            .transform_3d
            .rotation
            .keyframes()
            .map(|keys| keys.iter().map(|key| (key.frame, key.value[0])).collect()),
        GraphChannel::Rotation3DY => layer
            .transform_3d
            .rotation
            .keyframes()
            .map(|keys| keys.iter().map(|key| (key.frame, key.value[1])).collect()),
        GraphChannel::Rotation3DZ => layer
            .transform_3d
            .rotation
            .keyframes()
            .map(|keys| keys.iter().map(|key| (key.frame, key.value[2])).collect()),
        GraphChannel::Scale3DX => layer
            .transform_3d
            .scale
            .keyframes()
            .map(|keys| keys.iter().map(|key| (key.frame, key.value[0])).collect()),
        GraphChannel::Scale3DY => layer
            .transform_3d
            .scale
            .keyframes()
            .map(|keys| keys.iter().map(|key| (key.frame, key.value[1])).collect()),
        GraphChannel::Scale3DZ => layer
            .transform_3d
            .scale
            .keyframes()
            .map(|keys| keys.iter().map(|key| (key.frame, key.value[2])).collect()),
        GraphChannel::AudioVolume => match &layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Audio { volume, .. } => volume
                .keyframes()
                .map(|keys| keys.iter().map(|key| (key.frame, key.value)).collect()),
            kagari_vfx::core::timeline::LayerType::Video {
                audio_wav: Some(_),
                audio_volume,
                ..
            } => audio_volume
                .keyframes()
                .map(|keys| keys.iter().map(|key| (key.frame, key.value)).collect()),
            _ => None,
        },
        GraphChannel::TimeRemap => layer.time_remap.as_ref().and_then(|track| {
            track
                .keyframes()
                .map(|keys| keys.iter().map(|key| (key.frame, key.value)).collect())
        }),
        GraphChannel::CameraFov => camera
            .and_then(|camera| camera.fov_animation.as_ref())
            .and_then(|track| {
                track
                    .keyframes()
                    .map(|keys| keys.iter().map(|key| (key.frame, key.value)).collect())
            }),
        GraphChannel::CameraFocus => camera
            .and_then(|camera| camera.focus_distance_animation.as_ref())
            .and_then(|track| {
                track
                    .keyframes()
                    .map(|keys| keys.iter().map(|key| (key.frame, key.value)).collect())
            }),
        GraphChannel::CameraAperture => camera
            .and_then(|camera| camera.aperture_animation.as_ref())
            .and_then(|track| {
                track
                    .keyframes()
                    .map(|keys| keys.iter().map(|key| (key.frame, key.value)).collect())
            }),
        GraphChannel::CameraDofBlur => camera
            .and_then(|camera| camera.dof_max_blur_animation.as_ref())
            .and_then(|track| {
                track
                    .keyframes()
                    .map(|keys| keys.iter().map(|key| (key.frame, key.value)).collect())
            }),
        GraphChannel::CameraDofEnabled => camera
            .and_then(|camera| camera.dof_enabled_animation.as_ref())
            .and_then(|track| {
                track
                    .keyframes()
                    .map(|keys| keys.iter().map(|key| (key.frame, key.value)).collect())
            }),
        GraphChannel::LightIntensity => light
            .and_then(|light| light.intensity_animation.as_ref())
            .and_then(|track| {
                track
                    .keyframes()
                    .map(|keys| keys.iter().map(|key| (key.frame, key.value)).collect())
            }),
        GraphChannel::MaskFeather => mask_id
            .and_then(|id| layer.masks.iter().find(|mask| mask.id == id))
            .and_then(|mask| {
                mask.feather
                    .keyframes()
                    .map(|keys| keys.iter().map(|key| (key.frame, key.value)).collect())
            }),
        GraphChannel::MaskOpacity => mask_id
            .and_then(|id| layer.masks.iter().find(|mask| mask.id == id))
            .and_then(|mask| {
                mask.opacity
                    .keyframes()
                    .map(|keys| keys.iter().map(|key| (key.frame, key.value)).collect())
            }),
        GraphChannel::MaskExpansion => mask_id
            .and_then(|id| layer.masks.iter().find(|mask| mask.id == id))
            .and_then(|mask| {
                mask.expansion
                    .keyframes()
                    .map(|keys| keys.iter().map(|key| (key.frame, key.value)).collect())
            }),
        GraphChannel::ShapeWidth
        | GraphChannel::ShapeHeight
        | GraphChannel::ShapeCornerRadius
        | GraphChannel::ShapePoints
        | GraphChannel::ShapeInnerRadius
        | GraphChannel::ShapeOuterRadius
        | GraphChannel::ShapeSides
        | GraphChannel::ShapeRadius => shape_parameter_track(layer, channel).and_then(|track| {
            track
                .keyframes()
                .map(|keys| keys.iter().map(|key| (key.frame, key.value)).collect())
        }),
        GraphChannel::EffectParameter => scalar_effect_track(layer, effect_id, effect_parameter_id)
            .and_then(|track| {
                track
                    .keyframes()
                    .map(|keys| keys.iter().map(|key| (key.frame, key.value)).collect())
            }),
        GraphChannel::EffectColorRed
        | GraphChannel::EffectColorGreen
        | GraphChannel::EffectColorBlue
        | GraphChannel::EffectColorAlpha => {
            let component = effect_color_component(channel).unwrap_or(0);
            effect_color_track(layer, effect_id, effect_parameter_id).and_then(|track| {
                track.keyframes().map(|keys| {
                    keys.iter()
                        .map(|key| (key.frame, key.value[component]))
                        .collect()
                })
            })
        }
    };
    let mut keys = keys.unwrap_or_default();
    keys.sort_by_key(|key| key.0);
    keys
}

pub(crate) fn mask_mode_label(mode: kagari_vfx::core::mask::MaskMode) -> &'static str {
    use kagari_vfx::core::mask::MaskMode;
    match mode {
        MaskMode::Add => "加算",
        MaskMode::Subtract => "減算",
        MaskMode::Intersect => "交差",
        MaskMode::Lighten => "明るく",
        MaskMode::Darken => "暗く",
        MaskMode::Difference => "差分",
        MaskMode::None => "なし",
    }
}

pub(crate) fn blend_mode_label(mode: kagari_vfx::core::timeline::BlendMode) -> &'static str {
    use kagari_vfx::core::timeline::BlendMode::*;
    match mode {
        Normal => "通常",
        Multiply => "乗算",
        Screen => "スクリーン",
        Overlay => "オーバーレイ",
        Add => "加算",
        Darken => "比較（暗）",
        Lighten => "比較（明）",
        SoftLight => "ソフトライト",
        HardLight => "ハードライト",
        Difference => "差の絶対値",
        Exclusion => "除外",
        Divide => "除算",
        Subtract => "減算",
        ColorBurn => "焼き込みカラー",
        LinearBurn => "焼き込みリニア",
        VividLight => "ビビッドライト",
        ColorDodge => "覆い焼きカラー",
        LinearDodge => "覆い焼きリニア",
        Color => "カラー",
        Hue => "色相",
        Saturation => "彩度",
        Luminosity => "輝度",
        StencilAlpha => "ステンシルα",
        StencilLuma => "ステンシル輝度",
        SilhouetteAlpha => "シルエットα",
        SilhouetteLuma => "シルエット輝度",
        Behind => "背面",
        AlphaAdd => "α加算",
        LinearLight => "リニアライト",
    }
}

pub(crate) fn auto_orient_label(
    mode: kagari_vfx::core::auto_orient::AutoOrientMode,
) -> &'static str {
    match mode {
        kagari_vfx::core::auto_orient::AutoOrientMode::Off => "なし",
        kagari_vfx::core::auto_orient::AutoOrientMode::OrientAlongPath => "パスに沿う",
        kagari_vfx::core::auto_orient::AutoOrientMode::OrientTowardsPoint { .. } => "点を向く",
    }
}

pub(crate) fn layer_label_color_label(
    color: kagari_vfx::core::timeline::LabelColor,
) -> &'static str {
    use kagari_vfx::core::timeline::LabelColor;
    match color {
        LabelColor::None => "なし",
        LabelColor::Red => "赤",
        LabelColor::Yellow => "黄",
        LabelColor::Aqua => "水色",
        LabelColor::Pink => "桃",
        LabelColor::Lavender => "藤",
        LabelColor::Peach => "橙",
        LabelColor::Sea => "緑",
        LabelColor::Blue => "青",
        LabelColor::Purple => "紫",
    }
}

pub(crate) fn layer_label_color_badge(
    color: kagari_vfx::core::timeline::LabelColor,
) -> &'static str {
    use kagari_vfx::core::timeline::LabelColor;
    match color {
        LabelColor::None => "▫",
        LabelColor::Red => "🔴",
        LabelColor::Yellow => "🟡",
        LabelColor::Aqua => "🔷",
        LabelColor::Pink => "🌸",
        LabelColor::Lavender => "🟣",
        LabelColor::Peach => "🟠",
        LabelColor::Sea => "🟢",
        LabelColor::Blue => "🔵",
        LabelColor::Purple => "🟪",
    }
}

pub(crate) fn graph_channel_value(
    layer: &kagari_vfx::core::timeline::Layer,
    camera: Option<&kagari_vfx::core::timeline::Camera3D>,
    light: Option<&kagari_vfx::core::timeline::Light3D>,
    mask_id: Option<&str>,
    effect_id: Option<&str>,
    effect_parameter_id: Option<&str>,
    channel: GraphChannel,
    frame: u32,
) -> f32 {
    match channel {
        GraphChannel::PositionX => layer.transform.position.evaluate(frame)[0],
        GraphChannel::PositionY => layer.transform.position.evaluate(frame)[1],
        GraphChannel::AnchorX => layer.transform.anchor_point.evaluate(frame)[0],
        GraphChannel::AnchorY => layer.transform.anchor_point.evaluate(frame)[1],
        GraphChannel::ScaleX => layer.transform.scale.evaluate(frame)[0],
        GraphChannel::ScaleY => layer.transform.scale.evaluate(frame)[1],
        GraphChannel::Rotation => layer.transform.rotation.evaluate(frame),
        GraphChannel::Opacity => layer.transform.opacity.evaluate(frame),
        GraphChannel::Position3DX => layer.transform_3d.position.evaluate(frame)[0],
        GraphChannel::Position3DY => layer.transform_3d.position.evaluate(frame)[1],
        GraphChannel::Position3DZ => layer.transform_3d.position.evaluate(frame)[2],
        GraphChannel::Rotation3DX => layer.transform_3d.rotation.evaluate(frame)[0],
        GraphChannel::Rotation3DY => layer.transform_3d.rotation.evaluate(frame)[1],
        GraphChannel::Rotation3DZ => layer.transform_3d.rotation.evaluate(frame)[2],
        GraphChannel::Scale3DX => layer.transform_3d.scale.evaluate(frame)[0],
        GraphChannel::Scale3DY => layer.transform_3d.scale.evaluate(frame)[1],
        GraphChannel::Scale3DZ => layer.transform_3d.scale.evaluate(frame)[2],
        GraphChannel::AudioVolume => match &layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Audio { volume, .. } => volume.evaluate(frame),
            kagari_vfx::core::timeline::LayerType::Video {
                audio_wav: Some(_),
                audio_volume,
                ..
            } => audio_volume.evaluate(frame),
            _ => 0.0,
        },
        GraphChannel::TimeRemap => layer
            .time_remap
            .as_ref()
            .map(|track| track.evaluate(frame))
            .unwrap_or(frame as f32),
        GraphChannel::CameraFov => camera.map(|camera| camera.fov_at(frame)).unwrap_or(0.0),
        GraphChannel::CameraFocus => camera
            .map(|camera| camera.focus_distance_at(frame))
            .unwrap_or(0.0),
        GraphChannel::CameraAperture => camera
            .map(|camera| camera.aperture_at(frame))
            .unwrap_or(0.0),
        GraphChannel::CameraDofBlur => camera
            .map(|camera| camera.dof_max_blur_at(frame))
            .unwrap_or(0.0),
        GraphChannel::CameraDofEnabled => camera
            .map(|camera| {
                if camera.dof_enabled_at(frame) {
                    1.0
                } else {
                    0.0
                }
            })
            .unwrap_or(0.0),
        GraphChannel::LightIntensity => light.map(|light| light.intensity_at(frame)).unwrap_or(0.0),
        GraphChannel::MaskFeather => mask_id
            .and_then(|id| layer.masks.iter().find(|mask| mask.id == id))
            .map(|mask| mask.feather.evaluate(frame))
            .unwrap_or(0.0),
        GraphChannel::MaskOpacity => mask_id
            .and_then(|id| layer.masks.iter().find(|mask| mask.id == id))
            .map(|mask| mask.opacity.evaluate(frame))
            .unwrap_or(0.0),
        GraphChannel::MaskExpansion => mask_id
            .and_then(|id| layer.masks.iter().find(|mask| mask.id == id))
            .map(|mask| mask.expansion.evaluate(frame))
            .unwrap_or(0.0),
        GraphChannel::ShapeWidth
        | GraphChannel::ShapeHeight
        | GraphChannel::ShapeCornerRadius
        | GraphChannel::ShapePoints
        | GraphChannel::ShapeInnerRadius
        | GraphChannel::ShapeOuterRadius
        | GraphChannel::ShapeSides
        | GraphChannel::ShapeRadius => shape_parameter_track(layer, channel)
            .map(|track| track.evaluate(frame))
            .unwrap_or(0.0),
        GraphChannel::EffectParameter => scalar_effect_track(layer, effect_id, effect_parameter_id)
            .map(|track| track.evaluate(frame))
            .unwrap_or(0.0),
        GraphChannel::EffectColorRed
        | GraphChannel::EffectColorGreen
        | GraphChannel::EffectColorBlue
        | GraphChannel::EffectColorAlpha => effect_color_component(channel)
            .and_then(|component| {
                effect_color_track(layer, effect_id, effect_parameter_id)
                    .map(|track| track.evaluate(frame)[component])
            })
            .unwrap_or(0.0),
    }
}

pub(crate) fn graph_value_range(keys: &[(u32, f32)]) -> (f32, f32) {
    let low = keys.iter().map(|key| key.1).fold(f32::INFINITY, f32::min);
    let high = keys
        .iter()
        .map(|key| key.1)
        .fold(f32::NEG_INFINITY, f32::max);
    if !low.is_finite() || !high.is_finite() {
        return (-1.0, 1.0);
    }
    let padding = ((high - low) * 0.15).max(high.abs().max(low.abs()).max(1.0) * 0.05);
    (low - padding, high + padding)
}

pub(crate) fn graph_interpolation_label(
    layer: &kagari_vfx::core::timeline::Layer,
    camera: Option<&kagari_vfx::core::timeline::Camera3D>,
    light: Option<&kagari_vfx::core::timeline::Light3D>,
    mask_id: Option<&str>,
    effect_id: Option<&str>,
    effect_parameter_id: Option<&str>,
    channel: GraphChannel,
    frame: u32,
) -> Option<String> {
    use kagari_vfx::core::keyframe::InterpolationType;
    let kind = match channel {
        GraphChannel::PositionX | GraphChannel::PositionY => {
            layer.transform.position.keyframes().and_then(|keys| {
                keys.iter()
                    .find(|key| key.frame == frame)
                    .map(|key| key.interpolation)
            })
        }
        GraphChannel::AnchorX | GraphChannel::AnchorY => {
            layer.transform.anchor_point.keyframes().and_then(|keys| {
                keys.iter()
                    .find(|key| key.frame == frame)
                    .map(|key| key.interpolation)
            })
        }
        GraphChannel::ScaleX | GraphChannel::ScaleY => {
            layer.transform.scale.keyframes().and_then(|keys| {
                keys.iter()
                    .find(|key| key.frame == frame)
                    .map(|key| key.interpolation)
            })
        }
        GraphChannel::Rotation => layer.transform.rotation.keyframes().and_then(|keys| {
            keys.iter()
                .find(|key| key.frame == frame)
                .map(|key| key.interpolation)
        }),
        GraphChannel::Opacity => layer.transform.opacity.keyframes().and_then(|keys| {
            keys.iter()
                .find(|key| key.frame == frame)
                .map(|key| key.interpolation)
        }),
        GraphChannel::Position3DX | GraphChannel::Position3DY | GraphChannel::Position3DZ => {
            layer.transform_3d.position.keyframes().and_then(|keys| {
                keys.iter()
                    .find(|key| key.frame == frame)
                    .map(|key| key.interpolation)
            })
        }
        GraphChannel::Rotation3DX | GraphChannel::Rotation3DY | GraphChannel::Rotation3DZ => {
            layer.transform_3d.rotation.keyframes().and_then(|keys| {
                keys.iter()
                    .find(|key| key.frame == frame)
                    .map(|key| key.interpolation)
            })
        }
        GraphChannel::Scale3DX | GraphChannel::Scale3DY | GraphChannel::Scale3DZ => {
            layer.transform_3d.scale.keyframes().and_then(|keys| {
                keys.iter()
                    .find(|key| key.frame == frame)
                    .map(|key| key.interpolation)
            })
        }
        GraphChannel::AudioVolume => match &layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Audio { volume, .. } => {
                volume.keyframes().and_then(|keys| {
                    keys.iter()
                        .find(|key| key.frame == frame)
                        .map(|key| key.interpolation)
                })
            }
            kagari_vfx::core::timeline::LayerType::Video {
                audio_wav: Some(_),
                audio_volume,
                ..
            } => audio_volume.keyframes().and_then(|keys| {
                keys.iter()
                    .find(|key| key.frame == frame)
                    .map(|key| key.interpolation)
            }),
            _ => None,
        },
        GraphChannel::TimeRemap => layer.time_remap.as_ref().and_then(|track| {
            track.keyframes().and_then(|keys| {
                keys.iter()
                    .find(|key| key.frame == frame)
                    .map(|key| key.interpolation)
            })
        }),
        GraphChannel::CameraFov => camera
            .and_then(|camera| camera.fov_animation.as_ref())
            .and_then(|track| {
                track.keyframes().and_then(|keys| {
                    keys.iter()
                        .find(|key| key.frame == frame)
                        .map(|key| key.interpolation)
                })
            }),
        GraphChannel::CameraFocus => camera
            .and_then(|camera| camera.focus_distance_animation.as_ref())
            .and_then(|track| {
                track.keyframes().and_then(|keys| {
                    keys.iter()
                        .find(|key| key.frame == frame)
                        .map(|key| key.interpolation)
                })
            }),
        GraphChannel::CameraAperture => camera
            .and_then(|camera| camera.aperture_animation.as_ref())
            .and_then(|track| {
                track.keyframes().and_then(|keys| {
                    keys.iter()
                        .find(|key| key.frame == frame)
                        .map(|key| key.interpolation)
                })
            }),
        GraphChannel::CameraDofBlur => camera
            .and_then(|camera| camera.dof_max_blur_animation.as_ref())
            .and_then(|track| {
                track.keyframes().and_then(|keys| {
                    keys.iter()
                        .find(|key| key.frame == frame)
                        .map(|key| key.interpolation)
                })
            }),
        GraphChannel::CameraDofEnabled => camera
            .and_then(|camera| camera.dof_enabled_animation.as_ref())
            .and_then(|track| {
                track.keyframes().and_then(|keys| {
                    keys.iter()
                        .find(|key| key.frame == frame)
                        .map(|key| key.interpolation)
                })
            }),
        GraphChannel::LightIntensity => light
            .and_then(|light| light.intensity_animation.as_ref())
            .and_then(|track| {
                track.keyframes().and_then(|keys| {
                    keys.iter()
                        .find(|key| key.frame == frame)
                        .map(|key| key.interpolation)
                })
            }),
        GraphChannel::MaskFeather => mask_id
            .and_then(|id| layer.masks.iter().find(|mask| mask.id == id))
            .and_then(|mask| {
                mask.feather.keyframes().and_then(|keys| {
                    keys.iter()
                        .find(|key| key.frame == frame)
                        .map(|key| key.interpolation)
                })
            }),
        GraphChannel::MaskOpacity => mask_id
            .and_then(|id| layer.masks.iter().find(|mask| mask.id == id))
            .and_then(|mask| {
                mask.opacity.keyframes().and_then(|keys| {
                    keys.iter()
                        .find(|key| key.frame == frame)
                        .map(|key| key.interpolation)
                })
            }),
        GraphChannel::MaskExpansion => mask_id
            .and_then(|id| layer.masks.iter().find(|mask| mask.id == id))
            .and_then(|mask| {
                mask.expansion.keyframes().and_then(|keys| {
                    keys.iter()
                        .find(|key| key.frame == frame)
                        .map(|key| key.interpolation)
                })
            }),
        GraphChannel::ShapeWidth
        | GraphChannel::ShapeHeight
        | GraphChannel::ShapeCornerRadius
        | GraphChannel::ShapePoints
        | GraphChannel::ShapeInnerRadius
        | GraphChannel::ShapeOuterRadius
        | GraphChannel::ShapeSides
        | GraphChannel::ShapeRadius => shape_parameter_track(layer, channel).and_then(|track| {
            track.keyframes().and_then(|keys| {
                keys.iter()
                    .find(|key| key.frame == frame)
                    .map(|key| key.interpolation)
            })
        }),
        GraphChannel::EffectParameter => scalar_effect_track(layer, effect_id, effect_parameter_id)
            .and_then(|track| {
                track.keyframes().and_then(|keys| {
                    keys.iter()
                        .find(|key| key.frame == frame)
                        .map(|key| key.interpolation)
                })
            }),
        GraphChannel::EffectColorRed
        | GraphChannel::EffectColorGreen
        | GraphChannel::EffectColorBlue
        | GraphChannel::EffectColorAlpha => {
            effect_color_track(layer, effect_id, effect_parameter_id).and_then(|track| {
                track.keyframes().and_then(|keys| {
                    keys.iter()
                        .find(|key| key.frame == frame)
                        .map(|key| key.interpolation)
                })
            })
        }
    }?;
    Some(
        match kind {
            InterpolationType::Linear => "直線",
            InterpolationType::Hold => "停止",
            InterpolationType::Bezier { .. } => "イージング",
        }
        .to_owned(),
    )
}
