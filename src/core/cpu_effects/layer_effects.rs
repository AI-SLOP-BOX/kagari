use super::*;

pub(super) fn apply(
    pixels: &mut [u8],
    effect_type: &EffectType,
    context: LayerEffectRenderContext<'_>,
) -> bool {
    if !matches!(
        effect_type,
        EffectType::Transform { .. }
            | EffectType::CameraLensBlur { .. }
            | EffectType::LinearColorKey { .. }
            | EffectType::ChannelCombiner { .. }
            | EffectType::TiltShift { .. }
    ) {
        return false;
    }
    let LayerEffectRenderContext {
        comp,
        layer_idx,
        width,
        height,
        frame,
        fps,
        light_screen,
    } = context;
    let _ = (comp, layer_idx, width, height, frame, fps, light_screen);
    match effect_type {
        EffectType::Transform {
            anchor_point,
            position,
            scale_width,
            scale_height,
            uniform_scale,
            skew_deg,
            skew_axis_deg,
            rotation_deg,
            opacity,
        } => {
            let params = crate::core::transform_effect::TransformEffectParams {
                anchor_point: anchor_point.evaluate(frame),
                position: position.evaluate(frame),
                scale_width: scale_width.evaluate(frame),
                scale_height: scale_height.evaluate(frame),
                uniform_scale: *uniform_scale,
                skew_deg: skew_deg.evaluate(frame),
                skew_axis_deg: skew_axis_deg.evaluate(frame),
                rotation_deg: rotation_deg.evaluate(frame),
                opacity: opacity.evaluate(frame),
            };
            let transformed = crate::core::transform_effect::apply_transform_effect(
                pixels, width, height, &params,
            );
            pixels.copy_from_slice(&transformed);
        }
        EffectType::CameraLensBlur {
            blur_radius,
            iris_blades,
            iris_rotation_deg,
            iris_roundness,
            highlight_gain,
            highlight_threshold,
        } => {
            let params = crate::core::camera_lens_blur::CameraLensBlurParams {
                blur_radius: blur_radius.evaluate(frame),
                iris_blades: *iris_blades,
                iris_rotation_deg: iris_rotation_deg.evaluate(frame),
                iris_roundness: iris_roundness.evaluate(frame),
                highlight_gain: highlight_gain.evaluate(frame),
                highlight_threshold: highlight_threshold.evaluate(frame),
            };
            let blurred = crate::core::camera_lens_blur::apply_camera_lens_blur(
                pixels, width, height, &params,
            );
            pixels.copy_from_slice(&blurred);
        }
        EffectType::LinearColorKey {
            key_color,
            match_mode,
            tolerance,
            softness,
        } => {
            let params = crate::core::linear_color_key::LinearColorKeyParams {
                key_color: key_color.evaluate(frame),
                match_mode: *match_mode,
                tolerance: tolerance.evaluate(frame),
                softness: softness.evaluate(frame),
            };
            crate::core::linear_color_key::apply_linear_color_key(pixels, width, height, &params);
        }
        EffectType::ChannelCombiner {
            from_channel,
            to_target,
            invert,
        } => {
            let params = crate::core::channel_combiner::ChannelCombinerParams {
                from_channel: *from_channel,
                to_target: *to_target,
                invert: *invert,
            };
            crate::core::channel_combiner::apply_channel_combiner(pixels, width, height, &params);
        }
        EffectType::TiltShift {
            focus_y,
            focus_height,
            max_blur,
        } => {
            use crate::core::ae_effects_pack_v19::apply_tilt_shift;
            apply_tilt_shift(
                pixels,
                width,
                height,
                focus_y.evaluate(frame),
                focus_height.evaluate(frame).max(1.0),
                max_blur.evaluate(frame).round().clamp(0.0, 32.0) as u32,
            );
        }
        _ => unreachable!(),
    }
    true
}
