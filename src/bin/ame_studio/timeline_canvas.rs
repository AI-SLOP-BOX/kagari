#![allow(clippy::too_many_arguments)]
use super::*;

pub(crate) fn draw_timeline(
    scene: &mut Scene,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    frame: u32,
    work_area: Option<(u32, u32)>,
    duration_frames: u32,
    timeline_start: f64,
    visible_frames: f64,
    layers: &[kagari_vfx::core::timeline::Layer],
    selected_layer_id: Option<&str>,
    selected_layer_marker_frame: Option<u32>,
    waveforms: &std::collections::HashMap<String, std::sync::Arc<CachedAudioWaveform>>,
    fps: u32,
) {
    let bounds = Rect::new(x, y, x + width, y + height);
    scene.fill(
        Fill::NonZero,
        Affine::IDENTITY,
        Color::from_rgb8(13, 15, 17),
        None,
        &bounds,
    );
    let visible_frames = visible_frames.max(1.0);
    let tracks_top = y + 22.0;
    let track_height = 28.0;
    let grid = Stroke::new(1.0);

    for tick in 0..=5 {
        let tick_x = x + width * tick as f64 / 5.0;
        scene.stroke(
            &grid,
            Affine::IDENTITY,
            Color::from_rgb8(38, 42, 47),
            None,
            &Line::new((tick_x, y), (tick_x, y + height)),
        );
    }
    for (index, layer) in layers.iter().enumerate() {
        let row_y = tracks_top + index as f64 * track_height;
        scene.stroke(
            &grid,
            Affine::IDENTITY,
            Color::from_rgb8(32, 36, 40),
            None,
            &Line::new((x, row_y), (x + width, row_y)),
        );
        let start =
            (f64::from(layer.in_frame.min(duration_frames)) - timeline_start) / visible_frames;
        let end = (f64::from(
            layer
                .out_frame
                .max(layer.in_frame.saturating_add(1))
                .min(duration_frames),
        ) - timeline_start)
            / visible_frames;
        if end <= 0.0 || start >= 1.0 {
            continue;
        }
        let clip_start_x = (x + width * start).max(x);
        let clip_end_x = (x + width * end).min(x + width).max(clip_start_x + 4.0);
        let clip = RoundedRect::new(
            clip_start_x,
            row_y + 3.0,
            clip_end_x,
            (row_y + track_height - 3.0).max(row_y + 8.0),
            3.0,
        );
        let selected = selected_layer_id == Some(layer.id.as_str());
        let color = if selected {
            Color::from_rgba8(220, 105, 48, 210)
        } else {
            if layer.label == kagari_vfx::core::timeline::LabelColor::None {
                Color::from_rgba8(83, 94, 102, if layer.visible { 175 } else { 75 })
            } else {
                let [red, green, blue] = layer.label.to_rgb();
                Color::from_rgba8(
                    (red.clamp(0.0, 1.0) * 255.0) as u8,
                    (green.clamp(0.0, 1.0) * 255.0) as u8,
                    (blue.clamp(0.0, 1.0) * 255.0) as u8,
                    if layer.visible { 175 } else { 75 },
                )
            }
        };
        scene.fill(Fill::NonZero, Affine::IDENTITY, color, None, &clip);
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            Color::from_rgba8(175, 185, 190, if layer.visible { 110 } else { 55 }),
            None,
            &clip,
        );
        draw_timeline_waveform(
            scene,
            layer,
            x,
            width,
            row_y + 3.0,
            row_y + track_height - 3.0,
            clip_start_x,
            clip_end_x,
            timeline_start,
            visible_frames,
            fps,
            waveforms,
        );
        if selected {
            for edge_x in [clip_start_x + 2.0, clip_end_x - 2.0] {
                scene.stroke(
                    &Stroke::new(2.0),
                    Affine::IDENTITY,
                    Color::from_rgb8(255, 222, 192),
                    None,
                    &Line::new((edge_x, row_y + 7.0), (edge_x, row_y + track_height - 7.0)),
                );
            }
        }

        // Show transform keyframes directly on the layer strip. Multiple
        // animated channels at the same frame share one marker so the strip
        // remains legible at dense timeline scales.
        for keyframe_frame in layer_keyframe_frames(layer) {
            if keyframe_frame < layer.in_frame || keyframe_frame > layer.out_frame {
                continue;
            }
            let marker_x = x + width
                * (f64::from(keyframe_frame.min(duration_frames)) - timeline_start)
                / visible_frames;
            let center_y = row_y + track_height * 0.5;
            let mut diamond = BezPath::new();
            diamond.move_to((marker_x, center_y - 4.0));
            diamond.line_to((marker_x + 4.0, center_y));
            diamond.line_to((marker_x, center_y + 4.0));
            diamond.line_to((marker_x - 4.0, center_y));
            diamond.close_path();
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                Color::from_rgb8(255, 218, 174),
                None,
                &diamond,
            );
        }

        for marker in &layer.markers {
            if marker.frame < layer.in_frame || marker.frame > layer.out_frame {
                continue;
            }
            let marker_x = x + width
                * (f64::from(marker.frame.min(duration_frames)) - timeline_start)
                / visible_frames;
            if marker_x < x || marker_x > x + width {
                continue;
            }
            let marker_y = row_y + 7.0;
            let mut pin = BezPath::new();
            pin.move_to((marker_x, marker_y - 4.5));
            pin.line_to((marker_x + 4.0, marker_y + 3.5));
            pin.line_to((marker_x - 4.0, marker_y + 3.5));
            pin.close_path();
            let color = if selected_layer_id == Some(layer.id.as_str())
                && selected_layer_marker_frame == Some(marker.frame)
            {
                Color::from_rgb8(228, 255, 246)
            } else {
                Color::from_rgb8(87, 224, 190)
            };
            scene.fill(Fill::NonZero, Affine::IDENTITY, color, None, &pin);
        }
    }

    if let Some((start_frame, end_frame)) = work_area {
        let start_x = x + width * (f64::from(start_frame) - timeline_start) / visible_frames;
        let end_x = x
            + width
                * (f64::from(end_frame.saturating_add(1)) - timeline_start)
                / visible_frames;
        let visible_left = start_x.max(x);
        let visible_right = end_x.min(x + width);
        if visible_right > visible_left {
            let work_area_rect = Rect::new(visible_left, y, visible_right, y + height);
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                Color::from_rgba8(245, 140, 75, 20),
                None,
                &work_area_rect,
            );
        }
        for edge_x in [start_x, end_x] {
            if (x..=x + width).contains(&edge_x) {
                scene.stroke(
                    &Stroke::new(1.5),
                    Affine::IDENTITY,
                    Color::from_rgba8(245, 140, 75, 210),
                    None,
                    &Line::new((edge_x, y), (edge_x, y + height)),
                );
                let handle = Rect::new(edge_x - 4.0, y + 1.0, edge_x + 4.0, y + 10.0);
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    Color::from_rgb8(245, 140, 75),
                    None,
                    &handle,
                );
            }
        }
    }

    let playhead_x = x + width * (f64::from(frame.min(duration_frames.max(1))) - timeline_start)
        / visible_frames;
    if playhead_x >= x && playhead_x <= x + width {
        scene.stroke(
            &Stroke::new(1.8),
            Affine::IDENTITY,
            Color::from_rgb8(255, 116, 28),
            None,
            &Line::new((playhead_x, y), (playhead_x, y + height)),
        );
        let mut marker = BezPath::new();
        marker.move_to((playhead_x - 5.0, y));
        marker.line_to((playhead_x + 5.0, y));
        marker.line_to((playhead_x, y + 8.0));
        marker.close_path();
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            Color::from_rgb8(255, 116, 28),
            None,
            &marker,
        );
    }
}

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

pub(crate) fn auto_orient_label(mode: kagari_vfx::core::auto_orient::AutoOrientMode) -> &'static str {
    match mode {
        kagari_vfx::core::auto_orient::AutoOrientMode::Off => "なし",
        kagari_vfx::core::auto_orient::AutoOrientMode::OrientAlongPath => "パスに沿う",
        kagari_vfx::core::auto_orient::AutoOrientMode::OrientTowardsPoint { .. } => "点を向く",
    }
}

pub(crate) fn layer_label_color_label(color: kagari_vfx::core::timeline::LabelColor) -> &'static str {
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

pub(crate) fn layer_label_color_badge(color: kagari_vfx::core::timeline::LabelColor) -> &'static str {
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
        GraphChannel::LightIntensity => light
            .map(|light| light.intensity_at(frame))
            .unwrap_or(0.0),
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

pub(crate) fn cycle_track_interpolation<T: Clone>(
    track: &mut kagari_vfx::core::timeline::Animatable<T>,
    frame: u32,
) -> bool {
    use kagari_vfx::core::keyframe::{BezierControlPoint, InterpolationType};
    let Some(key) = track
        .keyframes_mut()
        .and_then(|keys| keys.iter_mut().find(|key| key.frame == frame))
    else {
        return false;
    };
    key.interpolation = match key.interpolation {
        InterpolationType::Linear => InterpolationType::Bezier {
            outgoing: BezierControlPoint {
                influence: 0.333,
                speed: 0.0,
            },
            incoming: BezierControlPoint {
                influence: 0.333,
                speed: 0.0,
            },
            custom_bezier: Some([0.25, 0.1, 0.25, 1.0]),
        },
        InterpolationType::Bezier { .. } => InterpolationType::Hold,
        InterpolationType::Hold => InterpolationType::Linear,
    };
    true
}

pub(crate) fn set_track_interpolation_preset<T: Clone>(
    track: &mut kagari_vfx::core::timeline::Animatable<T>,
    frame: u32,
    preset: &str,
) -> bool {
    use kagari_vfx::core::keyframe::{BezierControlPoint, InterpolationType};
    let Some(key) = track
        .keyframes_mut()
        .and_then(|keys| keys.iter_mut().find(|key| key.frame == frame))
    else {
        return false;
    };
    key.interpolation = match preset {
        "linear" => InterpolationType::Linear,
        "hold" => InterpolationType::Hold,
        "easy_ease" => InterpolationType::Bezier {
            outgoing: BezierControlPoint {
                influence: 0.333,
                speed: 0.0,
            },
            incoming: BezierControlPoint {
                influence: 0.333,
                speed: 0.0,
            },
            custom_bezier: Some([0.25, 0.1, 0.25, 1.0]),
        },
        "ease_in" => InterpolationType::Bezier {
            outgoing: BezierControlPoint {
                influence: 0.166,
                speed: 0.0,
            },
            incoming: BezierControlPoint {
                influence: 0.666,
                speed: 0.0,
            },
            custom_bezier: Some([0.42, 0.0, 1.0, 1.0]),
        },
        "ease_out" => InterpolationType::Bezier {
            outgoing: BezierControlPoint {
                influence: 0.666,
                speed: 0.0,
            },
            incoming: BezierControlPoint {
                influence: 0.166,
                speed: 0.0,
            },
            custom_bezier: Some([0.0, 0.0, 0.58, 1.0]),
        },
        "overshoot" => InterpolationType::Bezier {
            outgoing: BezierControlPoint {
                influence: 0.68,
                speed: 0.0,
            },
            incoming: BezierControlPoint {
                influence: 0.265,
                speed: 0.0,
            },
            custom_bezier: Some([0.68, -0.4, 0.265, 1.4]),
        },
        "bounce" => InterpolationType::Bezier {
            outgoing: BezierControlPoint {
                influence: 0.5,
                speed: 0.3,
            },
            incoming: BezierControlPoint {
                influence: 0.7,
                speed: 0.1,
            },
            custom_bezier: Some([0.5, 0.3, 0.7, 0.1]),
        },
        "sine" => InterpolationType::Bezier {
            outgoing: BezierControlPoint {
                influence: 0.37,
                speed: 0.0,
            },
            incoming: BezierControlPoint {
                influence: 0.63,
                speed: 0.0,
            },
            custom_bezier: Some([0.37, 0.0, 0.63, 1.0]),
        },
        _ => return false,
    };
    true
}

pub(crate) fn set_graph_interpolation_preset(
    layer: &mut kagari_vfx::core::timeline::Layer,
    camera: Option<&mut kagari_vfx::core::timeline::Camera3D>,
    light: Option<&mut kagari_vfx::core::timeline::Light3D>,
    mask_id: Option<&str>,
    effect_id: Option<&str>,
    effect_parameter_id: Option<&str>,
    channel: GraphChannel,
    frame: u32,
    preset: &str,
) -> bool {
    match channel {
        GraphChannel::PositionX | GraphChannel::PositionY => {
            set_track_interpolation_preset(&mut layer.transform.position, frame, preset)
        }
        GraphChannel::AnchorX | GraphChannel::AnchorY => {
            set_track_interpolation_preset(&mut layer.transform.anchor_point, frame, preset)
        }
        GraphChannel::ScaleX | GraphChannel::ScaleY => {
            set_track_interpolation_preset(&mut layer.transform.scale, frame, preset)
        }
        GraphChannel::Rotation => set_track_interpolation_preset(&mut layer.transform.rotation, frame, preset),
        GraphChannel::Opacity => set_track_interpolation_preset(&mut layer.transform.opacity, frame, preset),
        GraphChannel::Position3DX | GraphChannel::Position3DY | GraphChannel::Position3DZ => {
            set_track_interpolation_preset(&mut layer.transform_3d.position, frame, preset)
        }
        GraphChannel::Rotation3DX | GraphChannel::Rotation3DY | GraphChannel::Rotation3DZ => {
            set_track_interpolation_preset(&mut layer.transform_3d.rotation, frame, preset)
        }
        GraphChannel::Scale3DX | GraphChannel::Scale3DY | GraphChannel::Scale3DZ => {
            set_track_interpolation_preset(&mut layer.transform_3d.scale, frame, preset)
        }
        GraphChannel::AudioVolume => match &mut layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Audio { volume, .. } => {
                set_track_interpolation_preset(volume, frame, preset)
            }
            kagari_vfx::core::timeline::LayerType::Video {
                audio_wav: Some(_),
                audio_volume,
                ..
            } => set_track_interpolation_preset(audio_volume, frame, preset),
            _ => false,
        },
        GraphChannel::TimeRemap => layer
            .time_remap
            .as_mut()
            .is_some_and(|track| set_track_interpolation_preset(track, frame, preset)),
        GraphChannel::CameraFov => camera
            .and_then(|camera| camera.fov_animation.as_mut())
            .is_some_and(|track| set_track_interpolation_preset(track, frame, preset)),
        GraphChannel::CameraFocus => camera
            .and_then(|camera| camera.focus_distance_animation.as_mut())
            .is_some_and(|track| set_track_interpolation_preset(track, frame, preset)),
        GraphChannel::CameraAperture => camera
            .and_then(|camera| camera.aperture_animation.as_mut())
            .is_some_and(|track| set_track_interpolation_preset(track, frame, preset)),
        GraphChannel::CameraDofBlur => camera
            .and_then(|camera| camera.dof_max_blur_animation.as_mut())
            .is_some_and(|track| set_track_interpolation_preset(track, frame, preset)),
        GraphChannel::CameraDofEnabled => camera
            .and_then(|camera| camera.dof_enabled_animation.as_mut())
            .is_some_and(|track| set_track_interpolation_preset(track, frame, preset)),
        GraphChannel::LightIntensity => light
            .and_then(|light| light.intensity_animation.as_mut())
            .is_some_and(|track| set_track_interpolation_preset(track, frame, preset)),
        GraphChannel::MaskFeather => mask_id
            .and_then(|id| layer.masks.iter_mut().find(|mask| mask.id == id))
            .is_some_and(|mask| set_track_interpolation_preset(&mut mask.feather, frame, preset)),
        GraphChannel::MaskOpacity => mask_id
            .and_then(|id| layer.masks.iter_mut().find(|mask| mask.id == id))
            .is_some_and(|mask| set_track_interpolation_preset(&mut mask.opacity, frame, preset)),
        GraphChannel::MaskExpansion => mask_id
            .and_then(|id| layer.masks.iter_mut().find(|mask| mask.id == id))
            .is_some_and(|mask| set_track_interpolation_preset(&mut mask.expansion, frame, preset)),
        GraphChannel::ShapeWidth
        | GraphChannel::ShapeHeight
        | GraphChannel::ShapeCornerRadius
        | GraphChannel::ShapePoints
        | GraphChannel::ShapeInnerRadius
        | GraphChannel::ShapeOuterRadius
        | GraphChannel::ShapeSides
        | GraphChannel::ShapeRadius => shape_parameter_track_mut(layer, channel)
            .is_some_and(|track| set_track_interpolation_preset(track, frame, preset)),
        GraphChannel::EffectParameter => {
            scalar_effect_track_mut(layer, effect_id, effect_parameter_id)
                .is_some_and(|track| set_track_interpolation_preset(track, frame, preset))
        }
        GraphChannel::EffectColorRed
        | GraphChannel::EffectColorGreen
        | GraphChannel::EffectColorBlue
        | GraphChannel::EffectColorAlpha => {
            effect_color_track_mut(layer, effect_id, effect_parameter_id)
                .is_some_and(|track| set_track_interpolation_preset(track, frame, preset))
        }
    }
}

pub(crate) fn cycle_graph_interpolation(
    layer: &mut kagari_vfx::core::timeline::Layer,
    camera: Option<&mut kagari_vfx::core::timeline::Camera3D>,
    light: Option<&mut kagari_vfx::core::timeline::Light3D>,
    mask_id: Option<&str>,
    effect_id: Option<&str>,
    effect_parameter_id: Option<&str>,
    channel: GraphChannel,
    frame: u32,
) -> bool {
    match channel {
        GraphChannel::PositionX | GraphChannel::PositionY => {
            cycle_track_interpolation(&mut layer.transform.position, frame)
        }
        GraphChannel::AnchorX | GraphChannel::AnchorY => {
            cycle_track_interpolation(&mut layer.transform.anchor_point, frame)
        }
        GraphChannel::ScaleX | GraphChannel::ScaleY => {
            cycle_track_interpolation(&mut layer.transform.scale, frame)
        }
        GraphChannel::Rotation => cycle_track_interpolation(&mut layer.transform.rotation, frame),
        GraphChannel::Opacity => cycle_track_interpolation(&mut layer.transform.opacity, frame),
        GraphChannel::Position3DX | GraphChannel::Position3DY | GraphChannel::Position3DZ => {
            cycle_track_interpolation(&mut layer.transform_3d.position, frame)
        }
        GraphChannel::Rotation3DX | GraphChannel::Rotation3DY | GraphChannel::Rotation3DZ => {
            cycle_track_interpolation(&mut layer.transform_3d.rotation, frame)
        }
        GraphChannel::Scale3DX | GraphChannel::Scale3DY | GraphChannel::Scale3DZ => {
            cycle_track_interpolation(&mut layer.transform_3d.scale, frame)
        }
        GraphChannel::AudioVolume => match &mut layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Audio { volume, .. } => {
                cycle_track_interpolation(volume, frame)
            }
            kagari_vfx::core::timeline::LayerType::Video {
                audio_wav: Some(_),
                audio_volume,
                ..
            } => cycle_track_interpolation(audio_volume, frame),
            _ => false,
        },
        GraphChannel::TimeRemap => layer
            .time_remap
            .as_mut()
            .is_some_and(|track| cycle_track_interpolation(track, frame)),
        GraphChannel::CameraFov => camera
            .and_then(|camera| camera.fov_animation.as_mut())
            .is_some_and(|track| cycle_track_interpolation(track, frame)),
        GraphChannel::CameraFocus => camera
            .and_then(|camera| camera.focus_distance_animation.as_mut())
            .is_some_and(|track| cycle_track_interpolation(track, frame)),
        GraphChannel::CameraAperture => camera
            .and_then(|camera| camera.aperture_animation.as_mut())
            .is_some_and(|track| cycle_track_interpolation(track, frame)),
        GraphChannel::CameraDofBlur => camera
            .and_then(|camera| camera.dof_max_blur_animation.as_mut())
            .is_some_and(|track| cycle_track_interpolation(track, frame)),
        GraphChannel::CameraDofEnabled => camera
            .and_then(|camera| camera.dof_enabled_animation.as_mut())
            .is_some_and(|track| cycle_track_interpolation(track, frame)),
        GraphChannel::LightIntensity => light
            .and_then(|light| light.intensity_animation.as_mut())
            .is_some_and(|track| cycle_track_interpolation(track, frame)),
        GraphChannel::MaskFeather => mask_id
            .and_then(|id| layer.masks.iter_mut().find(|mask| mask.id == id))
            .is_some_and(|mask| cycle_track_interpolation(&mut mask.feather, frame)),
        GraphChannel::MaskOpacity => mask_id
            .and_then(|id| layer.masks.iter_mut().find(|mask| mask.id == id))
            .is_some_and(|mask| cycle_track_interpolation(&mut mask.opacity, frame)),
        GraphChannel::MaskExpansion => mask_id
            .and_then(|id| layer.masks.iter_mut().find(|mask| mask.id == id))
            .is_some_and(|mask| cycle_track_interpolation(&mut mask.expansion, frame)),
        GraphChannel::ShapeWidth
        | GraphChannel::ShapeHeight
        | GraphChannel::ShapeCornerRadius
        | GraphChannel::ShapePoints
        | GraphChannel::ShapeInnerRadius
        | GraphChannel::ShapeOuterRadius
        | GraphChannel::ShapeSides
        | GraphChannel::ShapeRadius => shape_parameter_track_mut(layer, channel)
            .is_some_and(|track| cycle_track_interpolation(track, frame)),
        GraphChannel::EffectParameter => {
            scalar_effect_track_mut(layer, effect_id, effect_parameter_id)
                .is_some_and(|track| cycle_track_interpolation(track, frame))
        }
        GraphChannel::EffectColorRed
        | GraphChannel::EffectColorGreen
        | GraphChannel::EffectColorBlue
        | GraphChannel::EffectColorAlpha => {
            effect_color_track_mut(layer, effect_id, effect_parameter_id)
                .is_some_and(|track| cycle_track_interpolation(track, frame))
        }
    }
}

pub(crate) fn add_graph_keyframe(
    layer: &mut kagari_vfx::core::timeline::Layer,
    camera: Option<&mut kagari_vfx::core::timeline::Camera3D>,
    light: Option<&mut kagari_vfx::core::timeline::Light3D>,
    mask_id: Option<&str>,
    effect_id: Option<&str>,
    effect_parameter_id: Option<&str>,
    channel: GraphChannel,
    frame: u32,
) {
    match channel {
        GraphChannel::PositionX | GraphChannel::PositionY => {
            add_current_keyframe(&mut layer.transform.position, frame);
        }
        GraphChannel::AnchorX | GraphChannel::AnchorY => {
            add_current_keyframe(&mut layer.transform.anchor_point, frame)
        }
        GraphChannel::ScaleX | GraphChannel::ScaleY => {
            add_current_keyframe(&mut layer.transform.scale, frame);
        }
        GraphChannel::Rotation => {
            add_current_keyframe(&mut layer.transform.rotation, frame);
        }
        GraphChannel::Opacity => {
            add_current_keyframe(&mut layer.transform.opacity, frame);
        }
        GraphChannel::Position3DX | GraphChannel::Position3DY | GraphChannel::Position3DZ => {
            add_current_keyframe(&mut layer.transform_3d.position, frame)
        }
        GraphChannel::Rotation3DX | GraphChannel::Rotation3DY | GraphChannel::Rotation3DZ => {
            add_current_keyframe(&mut layer.transform_3d.rotation, frame)
        }
        GraphChannel::Scale3DX | GraphChannel::Scale3DY | GraphChannel::Scale3DZ => {
            add_current_keyframe(&mut layer.transform_3d.scale, frame)
        }
        GraphChannel::AudioVolume => match &mut layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Audio { volume, .. } => {
                add_current_keyframe(volume, frame)
            }
            kagari_vfx::core::timeline::LayerType::Video {
                audio_wav: Some(_),
                audio_volume,
                ..
            } => add_current_keyframe(audio_volume, frame),
            _ => {}
        },
        GraphChannel::TimeRemap => {
            if let Some(track) = &mut layer.time_remap {
                add_current_keyframe(track, frame);
            }
        }
        GraphChannel::CameraFov => {
            if let Some(camera) = camera {
                let value = camera.fov_degrees;
                add_current_keyframe(
                    camera.fov_animation.get_or_insert_with(|| {
                        kagari_vfx::core::timeline::Animatable::new_constant(value)
                    }),
                    frame,
                );
            }
        }
        GraphChannel::CameraFocus => {
            if let Some(camera) = camera {
                let value = camera.focus_distance;
                add_current_keyframe(
                    camera.focus_distance_animation.get_or_insert_with(|| {
                        kagari_vfx::core::timeline::Animatable::new_constant(value)
                    }),
                    frame,
                );
            }
        }
        GraphChannel::CameraAperture => {
            if let Some(camera) = camera {
                let value = camera.aperture;
                add_current_keyframe(
                    camera.aperture_animation.get_or_insert_with(|| {
                        kagari_vfx::core::timeline::Animatable::new_constant(value)
                    }),
                    frame,
                );
            }
        }
        GraphChannel::CameraDofBlur => {
            if let Some(camera) = camera {
                let value = camera.dof_max_blur;
                add_current_keyframe(
                    camera.dof_max_blur_animation.get_or_insert_with(|| {
                        kagari_vfx::core::timeline::Animatable::new_constant(value)
                    }),
                    frame,
                );
            }
        }
        GraphChannel::CameraDofEnabled => {
            if let Some(camera) = camera {
                let value = if camera.dof_enabled_at(frame) {
                    1.0
                } else {
                    0.0
                };
                add_current_keyframe(
                    camera.dof_enabled_animation.get_or_insert_with(|| {
                        kagari_vfx::core::timeline::Animatable::new_constant(value)
                    }),
                    frame,
                );
            }
        }
        GraphChannel::LightIntensity => {
            if let Some(light) = light {
                add_current_keyframe(
                    light
                        .intensity_animation
                        .get_or_insert_with(|| {
                            kagari_vfx::core::timeline::Animatable::new_constant(light.intensity)
                        }),
                    frame,
                );
            }
        }
        GraphChannel::MaskFeather => {
            if let Some(mask) =
                mask_id.and_then(|id| layer.masks.iter_mut().find(|mask| mask.id == id))
            {
                add_current_keyframe(&mut mask.feather, frame);
            }
        }
        GraphChannel::MaskOpacity => {
            if let Some(mask) =
                mask_id.and_then(|id| layer.masks.iter_mut().find(|mask| mask.id == id))
            {
                add_current_keyframe(&mut mask.opacity, frame);
            }
        }
        GraphChannel::MaskExpansion => {
            if let Some(mask) =
                mask_id.and_then(|id| layer.masks.iter_mut().find(|mask| mask.id == id))
            {
                add_current_keyframe(&mut mask.expansion, frame);
            }
        }
        GraphChannel::ShapeWidth
        | GraphChannel::ShapeHeight
        | GraphChannel::ShapeCornerRadius
        | GraphChannel::ShapePoints
        | GraphChannel::ShapeInnerRadius
        | GraphChannel::ShapeOuterRadius
        | GraphChannel::ShapeSides
        | GraphChannel::ShapeRadius => {
            if let Some(track) = shape_parameter_track_mut(layer, channel) {
                add_current_keyframe(track, frame);
            }
        }
        GraphChannel::EffectParameter => {
            if let Some(track) = scalar_effect_track_mut(layer, effect_id, effect_parameter_id) {
                add_current_keyframe(track, frame);
            }
        }
        GraphChannel::EffectColorRed
        | GraphChannel::EffectColorGreen
        | GraphChannel::EffectColorBlue
        | GraphChannel::EffectColorAlpha => {
            if let Some(track) = effect_color_track_mut(layer, effect_id, effect_parameter_id) {
                add_current_keyframe(track, frame);
            }
        }
    }
}

pub(crate) fn remove_graph_keyframe(
    layer: &mut kagari_vfx::core::timeline::Layer,
    camera: Option<&mut kagari_vfx::core::timeline::Camera3D>,
    light: Option<&mut kagari_vfx::core::timeline::Light3D>,
    mask_id: Option<&str>,
    effect_id: Option<&str>,
    effect_parameter_id: Option<&str>,
    channel: GraphChannel,
    frame: u32,
) -> bool {
    match channel {
        GraphChannel::PositionX | GraphChannel::PositionY => {
            layer.transform.position.remove_keyframe(frame)
        }
        GraphChannel::AnchorX | GraphChannel::AnchorY => {
            layer.transform.anchor_point.remove_keyframe(frame)
        }
        GraphChannel::ScaleX | GraphChannel::ScaleY => layer.transform.scale.remove_keyframe(frame),
        GraphChannel::Rotation => layer.transform.rotation.remove_keyframe(frame),
        GraphChannel::Opacity => layer.transform.opacity.remove_keyframe(frame),
        GraphChannel::Position3DX | GraphChannel::Position3DY | GraphChannel::Position3DZ => {
            layer.transform_3d.position.remove_keyframe(frame)
        }
        GraphChannel::Rotation3DX | GraphChannel::Rotation3DY | GraphChannel::Rotation3DZ => {
            layer.transform_3d.rotation.remove_keyframe(frame)
        }
        GraphChannel::Scale3DX | GraphChannel::Scale3DY | GraphChannel::Scale3DZ => {
            layer.transform_3d.scale.remove_keyframe(frame)
        }
        GraphChannel::AudioVolume => match &mut layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Audio { volume, .. } => {
                volume.remove_keyframe(frame)
            }
            kagari_vfx::core::timeline::LayerType::Video {
                audio_wav: Some(_),
                audio_volume,
                ..
            } => audio_volume.remove_keyframe(frame),
            _ => false,
        },
        GraphChannel::TimeRemap => layer
            .time_remap
            .as_mut()
            .is_some_and(|track| track.remove_keyframe(frame)),
        GraphChannel::CameraFov => camera.is_some_and(|camera| {
            let changed = camera
                .fov_animation
                .as_mut()
                .is_some_and(|track| track.remove_keyframe(frame));
            if changed {
                camera.fov_degrees = camera.fov_at(frame);
            }
            changed
        }),
        GraphChannel::CameraFocus => camera.is_some_and(|camera| {
            let changed = camera
                .focus_distance_animation
                .as_mut()
                .is_some_and(|track| track.remove_keyframe(frame));
            if changed {
                camera.focus_distance = camera.focus_distance_at(frame);
            }
            changed
        }),
        GraphChannel::CameraAperture => camera.is_some_and(|camera| {
            let changed = camera
                .aperture_animation
                .as_mut()
                .is_some_and(|track| track.remove_keyframe(frame));
            if changed {
                camera.aperture = camera.aperture_at(frame);
            }
            changed
        }),
        GraphChannel::CameraDofBlur => camera.is_some_and(|camera| {
            let changed = camera
                .dof_max_blur_animation
                .as_mut()
                .is_some_and(|track| track.remove_keyframe(frame));
            if changed {
                camera.dof_max_blur = camera.dof_max_blur_at(frame);
            }
            changed
        }),
        GraphChannel::CameraDofEnabled => camera.is_some_and(|camera| {
            let changed = camera
                .dof_enabled_animation
                .as_mut()
                .is_some_and(|track| track.remove_keyframe(frame));
            camera.dof_enabled = camera.dof_enabled_at(frame);
            changed
        }),
        GraphChannel::LightIntensity => light.is_some_and(|light| {
            let changed = light
                .intensity_animation
                .as_mut()
                .is_some_and(|track| track.remove_keyframe(frame));
            if changed {
                light.intensity = light.intensity_at(frame);
            }
            changed
        }),
        GraphChannel::MaskFeather => mask_id
            .and_then(|id| layer.masks.iter_mut().find(|mask| mask.id == id))
            .is_some_and(|mask| mask.feather.remove_keyframe(frame)),
        GraphChannel::MaskOpacity => mask_id
            .and_then(|id| layer.masks.iter_mut().find(|mask| mask.id == id))
            .is_some_and(|mask| mask.opacity.remove_keyframe(frame)),
        GraphChannel::MaskExpansion => mask_id
            .and_then(|id| layer.masks.iter_mut().find(|mask| mask.id == id))
            .is_some_and(|mask| mask.expansion.remove_keyframe(frame)),
        GraphChannel::ShapeWidth
        | GraphChannel::ShapeHeight
        | GraphChannel::ShapeCornerRadius
        | GraphChannel::ShapePoints
        | GraphChannel::ShapeInnerRadius
        | GraphChannel::ShapeOuterRadius
        | GraphChannel::ShapeSides
        | GraphChannel::ShapeRadius => shape_parameter_track_mut(layer, channel)
            .is_some_and(|track| track.remove_keyframe(frame)),
        GraphChannel::EffectParameter => {
            scalar_effect_track_mut(layer, effect_id, effect_parameter_id)
                .is_some_and(|track| track.remove_keyframe(frame))
        }
        GraphChannel::EffectColorRed
        | GraphChannel::EffectColorGreen
        | GraphChannel::EffectColorBlue
        | GraphChannel::EffectColorAlpha => {
            effect_color_track_mut(layer, effect_id, effect_parameter_id)
                .is_some_and(|track| track.remove_keyframe(frame))
        }
    }
}

pub(crate) fn update_graph_keyframe(
    layer: &mut kagari_vfx::core::timeline::Layer,
    camera: Option<&mut kagari_vfx::core::timeline::Camera3D>,
    light: Option<&mut kagari_vfx::core::timeline::Light3D>,
    mask_id: Option<&str>,
    effect_id: Option<&str>,
    effect_parameter_id: Option<&str>,
    channel: GraphChannel,
    source_frame: u32,
    target_frame: u32,
    value: f32,
) -> bool {
    if !value.is_finite() {
        return false;
    }
    let value = graph_clamp_value(channel, value);
    match channel {
        GraphChannel::PositionX | GraphChannel::PositionY => {
            let track = &mut layer.transform.position;
            if track.keyframes().is_none() {
                return false;
            }
            if source_frame != target_frame && !track.move_keyframe(source_frame, target_frame) {
                return false;
            }
            let mut position = track.evaluate(target_frame);
            position[if channel == GraphChannel::PositionX {
                0
            } else {
                1
            }] = value;
            track.set_value_at_frame(target_frame, position);
        }
        GraphChannel::AnchorX | GraphChannel::AnchorY => {
            let track = &mut layer.transform.anchor_point;
            if track.keyframes().is_none() {
                return false;
            }
            if source_frame != target_frame && !track.move_keyframe(source_frame, target_frame) {
                return false;
            }
            let mut anchor = track.evaluate(target_frame);
            anchor[if channel == GraphChannel::AnchorX {
                0
            } else {
                1
            }] = value;
            track.set_value_at_frame(target_frame, anchor);
        }
        GraphChannel::ScaleX | GraphChannel::ScaleY => {
            let track = &mut layer.transform.scale;
            if track.keyframes().is_none() {
                return false;
            }
            if source_frame != target_frame && !track.move_keyframe(source_frame, target_frame) {
                return false;
            }
            let mut scale = track.evaluate(target_frame);
            scale[if channel == GraphChannel::ScaleX {
                0
            } else {
                1
            }] = value;
            track.set_value_at_frame(target_frame, scale);
        }
        GraphChannel::Rotation => {
            let track = &mut layer.transform.rotation;
            if track.keyframes().is_none() {
                return false;
            }
            if source_frame != target_frame && !track.move_keyframe(source_frame, target_frame) {
                return false;
            }
            track.set_value_at_frame(target_frame, value);
        }
        GraphChannel::Opacity => {
            let track = &mut layer.transform.opacity;
            if track.keyframes().is_none() {
                return false;
            }
            if source_frame != target_frame && !track.move_keyframe(source_frame, target_frame) {
                return false;
            }
            track.set_value_at_frame(target_frame, value);
        }
        GraphChannel::Position3DX | GraphChannel::Position3DY | GraphChannel::Position3DZ => {
            let track = &mut layer.transform_3d.position;
            if track.keyframes().is_none() {
                return false;
            }
            if source_frame != target_frame && !track.move_keyframe(source_frame, target_frame) {
                return false;
            }
            let mut position = track.evaluate(target_frame);
            let axis = match channel {
                GraphChannel::Position3DX => 0,
                GraphChannel::Position3DY => 1,
                _ => 2,
            };
            position[axis] = value;
            track.set_value_at_frame(target_frame, position);
        }
        GraphChannel::Rotation3DX | GraphChannel::Rotation3DY | GraphChannel::Rotation3DZ => {
            let track = &mut layer.transform_3d.rotation;
            if track.keyframes().is_none() {
                return false;
            }
            if source_frame != target_frame && !track.move_keyframe(source_frame, target_frame) {
                return false;
            }
            let mut rotation = track.evaluate(target_frame);
            let axis = match channel {
                GraphChannel::Rotation3DX => 0,
                GraphChannel::Rotation3DY => 1,
                _ => 2,
            };
            rotation[axis] = value;
            track.set_value_at_frame(target_frame, rotation);
        }
        GraphChannel::Scale3DX | GraphChannel::Scale3DY | GraphChannel::Scale3DZ => {
            let track = &mut layer.transform_3d.scale;
            if track.keyframes().is_none() {
                return false;
            }
            if source_frame != target_frame && !track.move_keyframe(source_frame, target_frame) {
                return false;
            }
            let mut scale = track.evaluate(target_frame);
            let axis = match channel {
                GraphChannel::Scale3DX => 0,
                GraphChannel::Scale3DY => 1,
                _ => 2,
            };
            scale[axis] = value;
            track.set_value_at_frame(target_frame, scale);
        }
        GraphChannel::AudioVolume => {
            let volume = match &mut layer.layer_type {
                kagari_vfx::core::timeline::LayerType::Audio { volume, .. } => volume,
                kagari_vfx::core::timeline::LayerType::Video {
                    audio_wav: Some(_),
                    audio_volume,
                    ..
                } => audio_volume,
                _ => return false,
            };
            if update_scalar_graph_track(volume, source_frame, target_frame, value).is_none() {
                return false;
            }
        }
        GraphChannel::TimeRemap => {
            let Some(track) = &mut layer.time_remap else {
                return false;
            };
            if update_scalar_graph_track(track, source_frame, target_frame, value).is_none() {
                return false;
            }
        }
        GraphChannel::CameraFov => {
            let Some(camera) = camera else { return false };
            let Some(track) = &mut camera.fov_animation else {
                return false;
            };
            if update_scalar_graph_track(track, source_frame, target_frame, value).is_none() {
                return false;
            }
            camera.fov_degrees = track.evaluate(target_frame);
        }
        GraphChannel::CameraFocus => {
            let Some(camera) = camera else { return false };
            let Some(track) = &mut camera.focus_distance_animation else {
                return false;
            };
            if update_scalar_graph_track(track, source_frame, target_frame, value).is_none() {
                return false;
            }
            camera.focus_distance = track.evaluate(target_frame);
        }
        GraphChannel::CameraAperture => {
            let Some(camera) = camera else { return false };
            let Some(track) = &mut camera.aperture_animation else {
                return false;
            };
            if update_scalar_graph_track(track, source_frame, target_frame, value).is_none() {
                return false;
            }
            camera.aperture = track.evaluate(target_frame);
        }
        GraphChannel::CameraDofBlur => {
            let Some(camera) = camera else { return false };
            let Some(track) = &mut camera.dof_max_blur_animation else {
                return false;
            };
            if update_scalar_graph_track(track, source_frame, target_frame, value).is_none() {
                return false;
            }
            camera.dof_max_blur = track.evaluate(target_frame);
        }
        GraphChannel::CameraDofEnabled => {
            let Some(camera) = camera else { return false };
            let Some(track) = &mut camera.dof_enabled_animation else {
                return false;
            };
            if update_scalar_graph_track(track, source_frame, target_frame, value).is_none() {
                return false;
            }
            camera.dof_enabled = track.evaluate(target_frame) >= 0.5;
        }
        GraphChannel::LightIntensity => {
            let Some(light) = light else { return false };
            let Some(track) = &mut light.intensity_animation else {
                return false;
            };
            if update_scalar_graph_track(track, source_frame, target_frame, value).is_none() {
                return false;
            }
            light.intensity = track.evaluate(target_frame);
        }
        GraphChannel::MaskFeather | GraphChannel::MaskOpacity | GraphChannel::MaskExpansion => {
            let Some(mask) =
                mask_id.and_then(|id| layer.masks.iter_mut().find(|mask| mask.id == id))
            else {
                return false;
            };
            let track = match channel {
                GraphChannel::MaskFeather => &mut mask.feather,
                GraphChannel::MaskOpacity => &mut mask.opacity,
                _ => &mut mask.expansion,
            };
            if update_scalar_graph_track(track, source_frame, target_frame, value).is_none() {
                return false;
            }
        }
        GraphChannel::ShapeWidth
        | GraphChannel::ShapeHeight
        | GraphChannel::ShapeCornerRadius
        | GraphChannel::ShapePoints
        | GraphChannel::ShapeInnerRadius
        | GraphChannel::ShapeOuterRadius
        | GraphChannel::ShapeSides
        | GraphChannel::ShapeRadius => {
            let Some(track) = shape_parameter_track_mut(layer, channel) else {
                return false;
            };
            if update_scalar_graph_track(track, source_frame, target_frame, value).is_none() {
                return false;
            }
        }
        GraphChannel::EffectParameter => {
            let Some(track) = scalar_effect_track_mut(layer, effect_id, effect_parameter_id) else {
                return false;
            };
            if update_scalar_graph_track(track, source_frame, target_frame, value).is_none() {
                return false;
            }
        }
        GraphChannel::EffectColorRed
        | GraphChannel::EffectColorGreen
        | GraphChannel::EffectColorBlue
        | GraphChannel::EffectColorAlpha => {
            let Some(component) = effect_color_component(channel) else {
                return false;
            };
            let Some(track) = effect_color_track_mut(layer, effect_id, effect_parameter_id) else {
                return false;
            };
            if !update_color_component_graph_track(
                track,
                component,
                source_frame,
                target_frame,
                value,
            ) {
                return false;
            }
        }
    }
    true
}

pub(crate) fn graph_clamp_value(channel: GraphChannel, value: f32) -> f32 {
    match channel {
        GraphChannel::Opacity => value.clamp(0.0, 100.0),
        GraphChannel::CameraDofEnabled => value.clamp(0.0, 1.0),
        GraphChannel::LightIntensity => value.clamp(0.0, 500.0),
        GraphChannel::ScaleX
        | GraphChannel::ScaleY
        | GraphChannel::Scale3DX
        | GraphChannel::Scale3DY
        | GraphChannel::Scale3DZ => value.clamp(0.1, 1000.0),
        GraphChannel::Rotation => value.clamp(-36_000.0, 36_000.0),
        GraphChannel::Position3DX | GraphChannel::Position3DY | GraphChannel::Position3DZ => {
            value.clamp(-100_000.0, 100_000.0)
        }
        GraphChannel::Rotation3DX | GraphChannel::Rotation3DY | GraphChannel::Rotation3DZ => {
            value.clamp(-36_000.0, 36_000.0)
        }
        GraphChannel::AudioVolume => value.clamp(-48.0, 12.0),
        GraphChannel::TimeRemap => value.max(0.0),
        GraphChannel::CameraFov => value.clamp(1.0, 179.0),
        GraphChannel::CameraFocus | GraphChannel::CameraAperture => value.max(0.0),
        GraphChannel::CameraDofBlur => value.clamp(1.0, 64.0),
        GraphChannel::MaskFeather => value.clamp(0.0, 5000.0),
        GraphChannel::MaskOpacity => value.clamp(0.0, 100.0),
        GraphChannel::MaskExpansion => value.clamp(-5000.0, 5000.0),
        GraphChannel::ShapePoints => value.clamp(2.0, 100.0),
        GraphChannel::ShapeSides => value.clamp(3.0, 100.0),
        GraphChannel::ShapeWidth
        | GraphChannel::ShapeHeight
        | GraphChannel::ShapeCornerRadius
        | GraphChannel::ShapeInnerRadius
        | GraphChannel::ShapeOuterRadius
        | GraphChannel::ShapeRadius => value.clamp(0.0, 1_000_000.0),
        GraphChannel::EffectParameter => value.clamp(-1_000_000.0, 1_000_000.0),
        GraphChannel::EffectColorRed
        | GraphChannel::EffectColorGreen
        | GraphChannel::EffectColorBlue
        | GraphChannel::EffectColorAlpha => value.clamp(0.0, 1.0),
        GraphChannel::PositionX
        | GraphChannel::PositionY
        | GraphChannel::AnchorX
        | GraphChannel::AnchorY => value.clamp(-100_000.0, 100_000.0),
    }
}

pub(crate) fn update_scalar_graph_track(
    track: &mut kagari_vfx::core::timeline::Animatable<f32>,
    source_frame: u32,
    target_frame: u32,
    value: f32,
) -> Option<()> {
    track.keyframes()?;
    if source_frame != target_frame && !track.move_keyframe(source_frame, target_frame) {
        return None;
    }
    track.set_value_at_frame(target_frame, value);
    Some(())
}

pub(crate) fn find_video_preview_frame(
    compositions: &[kagari_vfx::core::timeline::Composition],
    source: &str,
) -> Option<std::path::PathBuf> {
    for composition in compositions {
        for layer in &composition.layers {
            if let kagari_vfx::core::timeline::LayerType::Video {
                source: layer_source,
                frames_dir,
                ..
            } = &layer.layer_type
            {
                if layer_source == source && !frames_dir.is_empty() {
                    for extension in ["webp", "png"] {
                        let frame = std::path::Path::new(frames_dir)
                            .join(format!("frame_00000.{extension}"));
                        if frame.is_file() {
                            return Some(frame);
                        }
                    }
                }
            }
        }
        if let Some(frame) = find_video_preview_frame(&composition.sub_compositions, source) {
            return Some(frame);
        }
    }
    None
}

pub(crate) fn draw_value_graph(
    scene: &mut Scene,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    frame: u32,
    duration_frames: u32,
    timeline_start: f64,
    visible_frames: f64,
    channel: GraphChannel,
    layer: Option<&kagari_vfx::core::timeline::Layer>,
    camera: Option<&kagari_vfx::core::timeline::Camera3D>,
    light: Option<&kagari_vfx::core::timeline::Light3D>,
    mask_id: Option<&str>,
    effect_id: Option<&str>,
    effect_parameter_id: Option<&str>,
    keys: &[(u32, f32)],
) {
    let bounds = Rect::new(x, y, x + width, y + height);
    scene.fill(
        Fill::NonZero,
        Affine::IDENTITY,
        Color::from_rgb8(13, 15, 17),
        None,
        &bounds,
    );
    let left = x + 14.0;
    let right = (x + width - 14.0).max(left + 1.0);
    let top = y + 12.0;
    let bottom = (y + height - 12.0).max(top + 1.0);
    let (min_value, max_value) = graph_value_range(keys);
    let value_span = f64::from((max_value - min_value).max(f32::EPSILON));
    let plot_height = bottom - top;
    let plot_width = right - left;
    let grid = Stroke::new(1.0);
    for division in 0..=4 {
        let fraction = division as f64 / 4.0;
        let grid_y = top + plot_height * fraction;
        let grid_x = left + plot_width * fraction;
        scene.stroke(
            &grid,
            Affine::IDENTITY,
            Color::from_rgb8(39, 44, 49),
            None,
            &Line::new((left, grid_y), (right, grid_y)),
        );
        scene.stroke(
            &grid,
            Affine::IDENTITY,
            Color::from_rgb8(31, 36, 41),
            None,
            &Line::new((grid_x, top), (grid_x, bottom)),
        );
    }
    if let (Some(layer), Some(first), Some(last)) = (layer, keys.first(), keys.last()) {
        if last.0 > first.0 {
            let sample_count = (last.0 - first.0).clamp(2, 120);
            let mut curve = BezPath::new();
            let mut area = BezPath::new();

            for sample in 0..=sample_count {
                let sample_frame = first.0 + (last.0 - first.0) * sample / sample_count;
                let value = graph_channel_value(
                    layer,
                    camera,
                    light,
                    mask_id,
                    effect_id,
                    effect_parameter_id,
                    channel,
                    sample_frame,
                );
                let px = left
                    + (f64::from(sample_frame) - timeline_start) / visible_frames.max(1.0)
                        * plot_width;
                let py =
                    bottom - (f64::from(value) - f64::from(min_value)) / value_span * plot_height;
                if sample == 0 {
                    curve.move_to((px, py));
                    area.move_to((px, bottom));
                    area.line_to((px, py));
                } else {
                    curve.line_to((px, py));
                    area.line_to((px, py));
                    if sample == sample_count {
                        area.line_to((px, bottom));
                        area.close_path();
                    }
                }
            }
            // 1. Soft glowing area fill under curve for instant visual comprehension of velocity/amplitude
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                Color::from_rgba8(255, 135, 55, 28),
                None,
                &area,
            );
            // 2. Outer curve glow
            scene.stroke(
                &Stroke::new(4.0),
                Affine::IDENTITY,
                Color::from_rgba8(255, 135, 55, 45),
                None,
                &curve,
            );
            // 3. Crisp vibrant main curve
            scene.stroke(
                &Stroke::new(2.2),
                Affine::IDENTITY,
                Color::from_rgb8(255, 148, 68),
                None,
                &curve,
            );
        }
    }
    // Keyframe diamonds with high-contrast inner core and accent border
    for (key_frame, value) in keys {
        let px =
            left + (f64::from(*key_frame) - timeline_start) / visible_frames.max(1.0) * plot_width;
        if px < left - 6.0 || px > right + 6.0 {
            continue;
        }
        let py = bottom - (f64::from(*value) - f64::from(min_value)) / value_span * plot_height;
        // Dynamic tangent handles if easing/bezier is active on this keyframe
        let is_bezier = layer.and_then(|l| {
            graph_interpolation_label(
                l,
                camera,
                light,
                mask_id,
                effect_id,
                effect_parameter_id,
                channel,
                *key_frame,
            )
        }).is_some_and(|label| label == "イージング");

        if is_bezier {
            let handle_len = 22.0;
            // In/out tangent line with subtle glow
            scene.stroke(
                &Stroke::new(1.2),
                Affine::IDENTITY,
                Color::from_rgba8(255, 180, 70, 160),
                None,
                &Line::new((px - handle_len, py), (px + handle_len, py)),
            );
            // In handle circle (left)
            let in_handle = Circle::new((px - handle_len, py), 2.5);
            scene.fill(Fill::NonZero, Affine::IDENTITY, Color::from_rgb8(255, 200, 100), None, &in_handle);
            scene.stroke(&Stroke::new(0.8), Affine::IDENTITY, Color::from_rgb8(13, 15, 17), None, &in_handle);

            // Out handle circle (right)
            let out_handle = Circle::new((px + handle_len, py), 2.5);
            scene.fill(Fill::NonZero, Affine::IDENTITY, Color::from_rgb8(255, 200, 100), None, &out_handle);
            scene.stroke(&Stroke::new(0.8), Affine::IDENTITY, Color::from_rgb8(13, 15, 17), None, &out_handle);
        } else {
            // Stub line for linear/hold
            scene.stroke(
                &Stroke::new(1.0),
                Affine::IDENTITY,
                Color::from_rgba8(160, 170, 185, 70),
                None,
                &Line::new((px - 6.0, py), (px + 6.0, py)),
            );
        }

        let mut diamond = BezPath::new();
        diamond.move_to((px, py - 5.5));
        diamond.line_to((px + 5.5, py));
        diamond.line_to((px, py + 5.5));
        diamond.line_to((px - 5.5, py));
        diamond.close_path();
        // Drop shadow for keyframe diamond
        scene.stroke(
            &Stroke::new(2.0),
            Affine::IDENTITY,
            Color::from_rgba8(10, 11, 13, 220),
            None,
            &diamond,
        );
        let diamond_color = if is_bezier {
            Color::from_rgb8(255, 215, 140)
        } else {
            Color::from_rgb8(210, 225, 240)
        };
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            diamond_color,
            None,
            &diamond,
        );
    }
    let playhead_x = left
        + (f64::from(frame.min(duration_frames)) - timeline_start) / visible_frames.max(1.0)
            * plot_width;
    if playhead_x >= left && playhead_x <= right {
        scene.stroke(
            &Stroke::new(1.5),
            Affine::IDENTITY,
            Color::from_rgb8(255, 116, 28),
            None,
            &Line::new((playhead_x, top), (playhead_x, bottom)),
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_timeline_waveform(
    scene: &mut Scene,
    layer: &kagari_vfx::core::timeline::Layer,
    timeline_x: f64,
    timeline_width: f64,
    clip_top: f64,
    clip_bottom: f64,
    clip_left: f64,
    clip_right: f64,
    timeline_start: f64,
    visible_frames: f64,
    fps: u32,
    waveforms: &std::collections::HashMap<String, std::sync::Arc<CachedAudioWaveform>>,
) {
    use kagari_vfx::core::timeline::LayerType;
    let (path, video_speed) = match &layer.layer_type {
        LayerType::Audio { path, .. } => (path.as_str(), 1.0),
        LayerType::Video {
            audio_wav: Some(path),
            speed,
            ..
        } => (path.as_str(), *speed),
        _ => return,
    };
    let Some(waveform) = waveforms.get(path) else {
        return;
    };
    if waveform.peaks.len() < 2 || waveform.duration_seconds <= 0.0 {
        return;
    }
    let center_y = (clip_top + clip_bottom) * 0.5;
    let amplitude = (clip_bottom - clip_top) * 0.40;
    let fps = f64::from(fps.max(1));
    let stroke = Stroke::new(1.0);
    let mut px = clip_left + 2.0;
    while px < clip_right - 2.0 {
        let fraction = ((px - timeline_x) / timeline_width.max(1.0)).clamp(0.0, 1.0);
        let comp_frame = timeline_start + fraction * visible_frames;
        let source_frame = match &layer.layer_type {
            LayerType::Video { .. } => {
                f64::from(layer.remap_frame_f32(comp_frame.round() as u32) * video_speed)
            }
            _ => comp_frame - f64::from(layer.in_frame),
        };
        let seconds = source_frame / fps;
        if seconds >= 0.0 && seconds < f64::from(waveform.duration_seconds) {
            let index = ((seconds / f64::from(waveform.duration_seconds))
                * waveform.peaks.len() as f64) as usize;
            let peak = waveform.peaks[index.min(waveform.peaks.len() - 1)].clamp(0.0, 1.0);
            let half_height = (f64::from(peak) * amplitude).max(1.0);
            scene.stroke(
                &stroke,
                Affine::IDENTITY,
                Color::from_rgba8(136, 218, 190, 205),
                None,
                &Line::new((px, center_y - half_height), (px, center_y + half_height)),
            );
        }
        px += 2.0;
    }
}

pub(crate) fn draw_asset_waveform(
    scene: &mut Scene,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    waveform: &CachedAudioWaveform,
) {
    if waveform.peaks.is_empty() || width <= 24.0 || height <= 16.0 {
        return;
    }
    let left = x + 12.0;
    let right = x + width - 12.0;
    let center_y = y + height * 0.5;
    let amplitude = (height * 0.42).max(1.0);
    let count = waveform
        .peaks
        .len()
        .min(((right - left) * 0.5).floor().max(1.0) as usize);
    let stroke = Stroke::new(1.25);
    for index in 0..count {
        let sample_index = index * waveform.peaks.len() / count;
        let peak = f64::from(waveform.peaks[sample_index].clamp(0.0, 1.0));
        let half_height = (peak * amplitude).max(1.0);
        let px = left + index as f64 / count.max(1) as f64 * (right - left);
        scene.stroke(
            &stroke,
            Affine::IDENTITY,
            Color::from_rgba8(136, 218, 190, 230),
            None,
            &Line::new((px, center_y - half_height), (px, center_y + half_height)),
        );
    }
}

pub(crate) fn format_timecode(frame: u32, fps: u32) -> String {
    let fps = fps.max(1);
    let total_seconds = frame / fps;
    format!(
        "{:02}:{:02}:{:02}:{:02}",
        total_seconds / 3600,
        (total_seconds / 60) % 60,
        total_seconds % 60,
        frame % fps
    )
}

pub(crate) fn format_duration_seconds(frames: u32, fps: u32) -> String {
    let seconds = f64::from(frames) / f64::from(fps.max(1));
    if seconds.fract().abs() < 0.000_1 {
        format!("{seconds:.0}")
    } else {
        format!("{seconds:.2}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
    }
}

pub(crate) fn format_timeline_seconds(frame: u64, fps: u32) -> String {
    let total_seconds = frame / u64::from(fps.max(1));
    if total_seconds < 60 {
        format!("{total_seconds}s")
    } else {
        format!("{}:{:02}", total_seconds / 60, total_seconds % 60)
    }
}

pub(crate) fn model_preview_cache_key(path: &str) -> String {
    let revision = std::fs::metadata(path)
        .ok()
        .map(|metadata| {
            let modified = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|time| time.as_nanos())
                .unwrap_or_default();
            format!("{}:{modified}", metadata.len())
        })
        .unwrap_or_default();
    format!("model:{path}:{revision}")
}

pub(crate) fn asset_image_preview_cache_key(asset_id: &str, path: &str) -> String {
    let revision = std::fs::metadata(path)
        .ok()
        .map(|metadata| {
            let modified = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|time| time.as_nanos())
                .unwrap_or_default();
            format!("{}:{modified}", metadata.len())
        })
        .unwrap_or_default();
    format!("{asset_id}:{path}:{revision}")
}

pub(crate) fn render_asset_image_preview_pixels(
    cache: &mut kagari_vfx::core::image_cache::ImageCache,
    path: &str,
) -> Option<CachedAssetPreview> {
    const MAX_WIDTH: u32 = 640;
    const MAX_HEIGHT: u32 = 360;
    let image = cache.load_image(path)?;
    if image.width == 0 || image.height == 0 {
        return None;
    }
    let scale = (f64::from(MAX_WIDTH) / f64::from(image.width))
        .min(f64::from(MAX_HEIGHT) / f64::from(image.height))
        .min(1.0);
    let width = (f64::from(image.width) * scale).round().max(1.0) as u32;
    let height = (f64::from(image.height) * scale).round().max(1.0) as u32;
    let expected = (image.width as usize)
        .checked_mul(image.height as usize)?
        .checked_mul(4)?;
    if image.pixels.len() != expected {
        return None;
    }
    let mut pixels = vec![0; width as usize * height as usize * 4];
    for y in 0..height {
        let source_y = (u64::from(y) * u64::from(image.height) / u64::from(height)) as u32;
        for x in 0..width {
            let source_x = (u64::from(x) * u64::from(image.width) / u64::from(width)) as u32;
            let source_offset = ((source_y as usize * image.width as usize) + source_x as usize) * 4;
            let destination_offset = ((y as usize * width as usize) + x as usize) * 4;
            pixels[destination_offset..destination_offset + 4]
                .copy_from_slice(&image.pixels[source_offset..source_offset + 4]);
        }
    }
    Some(CachedAssetPreview {
        pixels: std::sync::Arc::new(pixels),
        width,
        height,
    })
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct FlowchartDragWireView {
    pub(crate) start: (f64, f64),
    pub(crate) current: (f64, f64),
    pub(crate) is_matte: bool,
}

pub(crate) fn draw_flowchart(
    scene: &mut Scene,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    comp: &kagari_vfx::core::timeline::Composition,
    selected_layer_id: Option<&str>,
    drag_wire: Option<FlowchartDragWireView>,
) {
    let bounds = Rect::new(x, y, x + width, y + height);
    // Dark canvas background with border
    scene.fill(
        Fill::NonZero,
        Affine::IDENTITY,
        Color::from_rgb8(13, 15, 17),
        None,
        &bounds,
    );
    scene.stroke(
        &Stroke::new(1.0),
        Affine::IDENTITY,
        Color::from_rgb8(28, 32, 38),
        None,
        &bounds,
    );

    // Subtle background dot / wire grid
    let grid_size = 28.0;
    let grid_stroke = Stroke::new(0.5);
    let mut gx = x + grid_size;
    while gx < x + width {
        scene.stroke(
            &grid_stroke,
            Affine::IDENTITY,
            Color::from_rgba8(36, 42, 50, 90),
            None,
            &Line::new((gx, y), (gx, y + height)),
        );
        gx += grid_size;
    }
    let mut gy = y + grid_size;
    while gy < y + height {
        scene.stroke(
            &grid_stroke,
            Affine::IDENTITY,
            Color::from_rgba8(36, 42, 50, 90),
            None,
            &Line::new((x, gy), (x + width, gy)),
        );
        gy += grid_size;
    }

    let layers_count = comp.layers.len();
    if layers_count == 0 {
        return;
    }

    let graph = kagari_vfx::core::flow_graph_model::FlowGraphModel::build_structure(comp);
    let node_w = 150.0;
    let node_h = 38.0;
    let col_spacing = 180.0;
    let row_spacing = 52.0;

    let origin_x = x + 24.0;
    let origin_y = y + 20.0;
    let positions = graph.positions_f64(origin_x, origin_y, col_spacing, row_spacing);

    // 1. Draw Connecting Wires with bezier curves
    for (idx, node) in graph.nodes.iter().enumerate() {
        if idx >= positions.len() {
            continue;
        }
        let (cx, cy) = positions[idx];
        let child_input = (cx, cy + node_h * 0.5);

        // Parent Connection Wire (Cyan curve)
        if let Some(parent_idx) = node.parent_index {
            if parent_idx < positions.len() {
                let (px, py) = positions[parent_idx];
                let parent_output = (px + node_w, py + node_h * 0.5);

                let mut path = BezPath::new();
                path.move_to(parent_output);
                let ctrl1 = (parent_output.0 + 40.0, parent_output.1);
                let ctrl2 = (child_input.0 - 40.0, child_input.1);
                path.curve_to(ctrl1, ctrl2, child_input);

                // Glow stroke
                scene.stroke(
                    &Stroke::new(3.0),
                    Affine::IDENTITY,
                    Color::from_rgba8(0, 210, 255, 60),
                    None,
                    &path,
                );
                // Core stroke
                scene.stroke(
                    &Stroke::new(1.5),
                    Affine::IDENTITY,
                    Color::from_rgb8(0, 220, 255),
                    None,
                    &path,
                );
            }
        }

        // Track Matte Connection Wire (Amber / Orange curve)
        if let Some(matte_idx) = node.matte_index {
            if matte_idx < positions.len() {
                let (mx, my) = positions[matte_idx];
                let matte_output = (mx + node_w, my + node_h * 0.5);

                let mut path = BezPath::new();
                path.move_to(matte_output);
                let ctrl1 = (matte_output.0 + 35.0, matte_output.1);
                let ctrl2 = (child_input.0 - 35.0, child_input.1);
                path.curve_to(ctrl1, ctrl2, child_input);

                // Glow stroke
                scene.stroke(
                    &Stroke::new(3.0),
                    Affine::IDENTITY,
                    Color::from_rgba8(255, 170, 0, 60),
                    None,
                    &path,
                );
                // Core stroke
                scene.stroke(
                    &Stroke::new(1.5),
                    Affine::IDENTITY,
                    Color::from_rgb8(255, 180, 20),
                    None,
                    &path,
                );
            }
        }
    }

    // 2. Draw Layer Card Nodes
    for (idx, layer) in comp.layers.iter().enumerate() {
        if idx >= positions.len() {
            continue;
        }
        let (nx, ny) = positions[idx];
        if ny + node_h < y || ny > y + height || nx + node_w < x || nx > x + width {
            continue;
        }

        let is_selected = selected_layer_id == Some(layer.id.as_str());
        let node_rect = RoundedRect::new(nx, ny, nx + node_w, ny + node_h, 5.0);

        // Card background fill
        let bg_color = if is_selected {
            Color::from_rgba8(26, 44, 58, 230)
        } else {
            Color::from_rgba8(22, 25, 30, 210)
        };
        scene.fill(Fill::NonZero, Affine::IDENTITY, bg_color, None, &node_rect);

        // Card border stroke
        let border_color = if is_selected {
            Color::from_rgb8(0, 220, 255)
        } else {
            Color::from_rgb8(45, 52, 62)
        };
        let border_width = if is_selected { 1.8 } else { 1.0 };
        scene.stroke(
            &Stroke::new(border_width),
            Affine::IDENTITY,
            border_color,
            None,
            &node_rect,
        );

        // Layer Label Color Bar (left accent)
        let label_color = if layer.label == kagari_vfx::core::timeline::LabelColor::None {
            Color::from_rgb8(80, 90, 100)
        } else {
            let [r, g, b] = layer.label.to_rgb();
            Color::from_rgb8(
                (r.clamp(0.0, 1.0) * 255.0) as u8,
                (g.clamp(0.0, 1.0) * 255.0) as u8,
                (b.clamp(0.0, 1.0) * 255.0) as u8,
            )
        };
        let color_bar = RoundedRect::new(nx + 4.0, ny + 6.0, nx + 9.0, ny + node_h - 6.0, 2.0);
        scene.fill(Fill::NonZero, Affine::IDENTITY, label_color, None, &color_bar);

        // Layer type badge & status pills inside node card
        let badge_x = nx + 16.0;
        let badge_y = ny + 10.0;
        let badge_w = 18.0;
        let badge_h = 18.0;
        let badge_rect = RoundedRect::new(badge_x, badge_y, badge_x + badge_w, badge_y + badge_h, 3.0);

        let (badge_bg, badge_stroke_color) = match &layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Text { .. } => (
                Color::from_rgba8(40, 100, 200, 180),
                Color::from_rgb8(80, 160, 255),
            ),
            kagari_vfx::core::timeline::LayerType::Shape { .. } => (
                Color::from_rgba8(180, 70, 20, 180),
                Color::from_rgb8(255, 120, 40),
            ),
            kagari_vfx::core::timeline::LayerType::AdjustmentLayer => (
                Color::from_rgba8(120, 40, 180, 180),
                Color::from_rgb8(190, 90, 255),
            ),
            kagari_vfx::core::timeline::LayerType::Particle { .. } => (
                Color::from_rgba8(20, 140, 120, 180),
                Color::from_rgb8(40, 220, 190),
            ),
            _ => (
                Color::from_rgba8(50, 60, 75, 180),
                Color::from_rgb8(100, 120, 145),
            ),
        };
        scene.fill(Fill::NonZero, Affine::IDENTITY, badge_bg, None, &badge_rect);
        scene.stroke(&Stroke::new(1.0), Affine::IDENTITY, badge_stroke_color, None, &badge_rect);

        // Draw geometric symbol inside badge:
        match &layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Text { .. } => {
                // "T" icon: horizontal bar + vertical stem
                let bar = Line::new((badge_x + 4.0, badge_y + 5.0), (badge_x + badge_w - 4.0, badge_y + 5.0));
                let stem = Line::new((badge_x + badge_w * 0.5, badge_y + 5.0), (badge_x + badge_w * 0.5, badge_y + badge_h - 4.0));
                scene.stroke(&Stroke::new(1.5), Affine::IDENTITY, Color::WHITE, None, &bar);
                scene.stroke(&Stroke::new(1.5), Affine::IDENTITY, Color::WHITE, None, &stem);
            }
            kagari_vfx::core::timeline::LayerType::Shape { .. } => {
                // Star/diamond polygon
                let diamond = RoundedRect::new(badge_x + 4.0, badge_y + 4.0, badge_x + badge_w - 4.0, badge_y + badge_h - 4.0, 2.0);
                scene.stroke(&Stroke::new(1.3), Affine::IDENTITY, Color::WHITE, None, &diamond);
            }
            _ => {
                // Dot icon
                let dot = Circle::new((badge_x + badge_w * 0.5, badge_y + badge_h * 0.5), 3.0);
                scene.fill(Fill::NonZero, Affine::IDENTITY, Color::WHITE, None, &dot);
            }
        }

        // Layer Name Pill & Details Bar
        let name_bar = RoundedRect::new(nx + 40.0, ny + 11.0, nx + node_w - 20.0, ny + 19.0, 2.0);
        let name_bar_color = if is_selected {
            Color::from_rgba8(220, 235, 255, 230)
        } else {
            Color::from_rgba8(140, 155, 175, 180)
        };
        scene.fill(Fill::NonZero, Affine::IDENTITY, name_bar_color, None, &name_bar);

        // Subtitle / Effects count indicator line
        let has_effects = layer.effects_enabled && !layer.effects.is_empty();
        let sub_w = if has_effects { 55.0 } else { 35.0 };
        let sub_bar = RoundedRect::new(nx + 40.0, ny + 23.0, nx + 40.0 + sub_w, ny + 27.0, 1.5);
        let sub_color = if has_effects {
            Color::from_rgba8(0, 210, 255, 180)
        } else {
            Color::from_rgba8(80, 95, 115, 140)
        };
        scene.fill(Fill::NonZero, Affine::IDENTITY, sub_color, None, &sub_bar);

        // 3D layer badge indicator (cyan cube corner)
        if layer.is_3d {
            let cube_dot = Circle::new((nx + node_w - 28.0, ny + 25.0), 2.5);
            scene.fill(Fill::NonZero, Affine::IDENTITY, Color::from_rgb8(0, 220, 255), None, &cube_dot);
        }

        // Input port pin (left side: Cyan)
        let in_pin = Circle::new((nx, ny + node_h * 0.5), 3.5);
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            Color::from_rgb8(0, 210, 255),
            None,
            &in_pin,
        );
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            Color::from_rgb8(13, 15, 17),
            None,
            &in_pin,
        );

        // Parent Output port pin (right side top: Gold / Yellow)
        let out_pin = Circle::new((nx + node_w, ny + 12.0), 3.5);
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            Color::from_rgb8(255, 210, 60),
            None,
            &out_pin,
        );
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            Color::from_rgb8(13, 15, 17),
            None,
            &out_pin,
        );

        // Matte Source Output port pin (right side bottom: Amber / Orange)
        let matte_pin = Circle::new((nx + node_w, ny + node_h - 12.0), 3.5);
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            Color::from_rgb8(255, 130, 20),
            None,
            &matte_pin,
        );
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            Color::from_rgb8(13, 15, 17),
            None,
            &matte_pin,
        );
    }

    // 3. Draw Interactive Dragging Wire (Active Pin-to-Pin connection)
    if let Some(FlowchartDragWireView { start, current, is_matte }) = drag_wire {
        let mut path = BezPath::new();
        path.move_to(start);
        let dx = (current.0 - start.0).abs().max(40.0) * 0.5;
        let ctrl1 = (start.0 + dx, start.1);
        let ctrl2 = (current.0 - dx, current.1);
        path.curve_to(ctrl1, ctrl2, current);

        let (glow_color, core_color, end_color) = if is_matte {
            (
                Color::from_rgba8(255, 130, 20, 90),
                Color::from_rgb8(255, 150, 40),
                Color::from_rgb8(255, 170, 60),
            )
        } else {
            (
                Color::from_rgba8(255, 230, 80, 80),
                Color::from_rgb8(255, 235, 100),
                Color::from_rgb8(255, 240, 120),
            )
        };

        scene.stroke(
            &Stroke::new(4.0),
            Affine::IDENTITY,
            glow_color,
            None,
            &path,
        );
        scene.stroke(
            &Stroke::new(2.0),
            Affine::IDENTITY,
            core_color,
            None,
            &path,
        );
        let end_point = Circle::new(current, 4.0);
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            end_color,
            None,
            &end_point,
        );
    }
}


