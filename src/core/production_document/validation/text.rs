use super::*;

fn in_range(value: f32, min: f32, max: f32) -> bool {
    value.is_finite() && (min..=max).contains(&value)
}

pub(super) fn text_formatting_is_valid(formatting: &crate::core::timeline::TextFormatting) -> bool {
    !formatting.font_family.trim().is_empty()
        && formatting.font_family.len() <= crate::core::font_rasterizer::MAX_FONT_FAMILY_NAME_BYTES
        && in_range(formatting.tracking, -100_000.0, 100_000.0)
        && in_range(formatting.leading, f32::MIN_POSITIVE, 1_000.0)
        && formatting
            .stroke_color
            .is_none_or(|color| color.iter().all(|value| value.is_finite()))
        && in_range(formatting.stroke_width, 0.0, 16_384.0)
        && formatting.alignment <= 3
        && in_range(formatting.box_width, 0.0, 1_000_000.0)
        && in_range(formatting.box_height, 0.0, 1_000_000.0)
}

fn scalar_animation_in_range(
    value: &crate::core::property::Animatable<f32>,
    min: f32,
    max: f32,
) -> bool {
    match value {
        crate::core::property::Animatable::Constant(value) => in_range(*value, min, max),
        crate::core::property::Animatable::Animated(keyframes) => {
            keyframes_are_strictly_ordered(keyframes)
                && keyframes
                    .iter()
                    .all(|keyframe| in_range(keyframe.value, min, max))
        }
    }
}

fn selector_is_valid(selector: &crate::core::text_animator::RangeSelector) -> bool {
    in_range(selector.start, 0.0, 100.0)
        && in_range(selector.end, 0.0, 100.0)
        && in_range(selector.offset, -100.0, 100.0)
        && in_range(selector.ease_high, 0.0, 100.0)
        && in_range(selector.ease_low, 0.0, 100.0)
        && selector.char_start >= -1
        && selector.char_end >= -1
        && selector.char_start <= 1_000_000
        && selector.char_end <= 1_000_000
        && !(selector.char_start >= 0
            && selector.char_end >= 0
            && selector.char_start > selector.char_end)
        && selector
            .offset_anim
            .as_ref()
            .is_none_or(|value| scalar_animation_in_range(value, -100.0, 100.0))
        && selector.expression.as_ref().is_none_or(|expression| {
            expression.len() <= ProductionDocument::MAX_ASSET_STRING_LENGTH
        })
}

fn standard_animator_is_valid(animator: &crate::core::text_animator::TextAnimatorSettings) -> bool {
    selector_is_valid(&animator.selector)
        && animator
            .position_offset
            .iter()
            .all(|value| in_range(*value, -10_000.0, 10_000.0))
        && animator
            .scale
            .iter()
            .all(|value| in_range(*value, 0.0, 10.0))
        && in_range(animator.opacity, -100.0, 100.0)
        && in_range(animator.tracking, -1_000.0, 1_000.0)
        && in_range(animator.rotation, -360.0, 360.0)
        && in_range(animator.blur_amount, 0.0, 500.0)
}

fn advanced_animator_is_valid(
    animator: &crate::core::text_animator_advanced::TextAnimatorAdvanced,
) -> bool {
    let advanced = &animator.advanced;
    selector_is_valid(&animator.selector)
        && animator
            .position
            .iter()
            .all(|value| in_range(*value, -10_000.0, 10_000.0))
        && animator
            .scale
            .iter()
            .all(|value| in_range(*value, 0.0, 10.0))
        && in_range(animator.opacity, -100.0, 100.0)
        && in_range(animator.tracking, -1_000.0, 1_000.0)
        && in_range(animator.rotation, -360.0, 360.0)
        && in_range(animator.blur, 0.0, 500.0)
        && advanced
            .anchor_point
            .iter()
            .all(|value| in_range(*value, -10_000.0, 10_000.0))
        && in_range(advanced.skew, -360.0, 360.0)
        && in_range(advanced.skew_axis, -360.0, 360.0)
        && advanced
            .fill_color
            .iter()
            .chain(advanced.stroke_color.iter())
            .flatten()
            .all(|value| value.is_finite())
        && in_range(advanced.stroke_width, 0.0, 16_384.0)
}

pub(super) fn layer_text_animators_are_valid(layer: &crate::core::timeline::Layer) -> bool {
    layer
        .text_animator
        .as_ref()
        .is_none_or(standard_animator_is_valid)
        && layer
            .text_animator_stack
            .as_ref()
            .is_none_or(|stack| stack.animators.iter().all(advanced_animator_is_valid))
}
