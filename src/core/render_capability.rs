//! Shared render capability checks (GPU vs CPU fallback).

use crate::core::timeline::{Composition, Layer, LayerType};

fn video_frame_blending_needed(layer: &Layer, frame: u32, fps: u32) -> bool {
    if !layer.frame_blending {
        return false;
    }
    let LayerType::Video {
        frames_dir,
        frame_count,
        speed,
        ..
    } = &layer.layer_type
    else {
        return false;
    };
    if *frame_count < 2 {
        return false;
    }

    let source_frame = match &layer.posterize_time {
        Some(settings) if settings.enabled => layer.effective_render_frame(frame, fps) as f32,
        _ => layer.remap_frame_f32(frame),
    };
    let source_position =
        crate::core::timeline::video_sequence_frame(source_frame, *speed, frames_dir, fps);
    let last_frame = frame_count.saturating_sub(1);
    let first_frame = (source_position.floor() as u32).min(last_frame);
    first_frame < last_frame && source_position.fract() > f32::EPSILON
}

/// Check if a composition requires CPU (software) rendering at a given frame.
/// Returns true if the composition uses any features unsupported by the GPU renderer.
pub fn software_preview_required(comp: &Composition, frame: u32) -> bool {
    software_preview_reason(comp, frame).is_some()
}

/// Explains why software preview is required at a given frame, or None if GPU is capable.
pub fn software_preview_reason(comp: &Composition, frame: u32) -> Option<&'static str> {
    if comp.blend_linear {
        return Some("linear-light compositing requires CPU renderer");
    }
    if comp.working_color_space != crate::core::color_science::WorkingColorSpace::Rec709 {
        return Some("working color-space conversion requires CPU renderer");
    }
    if comp.dither_output {
        return Some("output dithering requires CPU renderer");
    }
    let dof_enabled = comp.resolve_camera().dof_enabled_at(frame);

    for layer in comp.layers.iter().filter(|l| l.is_active(frame)) {
        if video_frame_blending_needed(layer, frame, comp.fps) {
            return Some("video frame blending requires CPU renderer");
        }
        if !layer.paint_strokes.is_empty() {
            return Some("paint strokes require CPU renderer");
        }
        if layer.style.stroke.enabled {
            return Some("layer-style strokes require CPU renderer");
        }
        if !layer.puppet_pins.is_empty() {
            return Some("puppet pins require CPU renderer");
        }
        if layer.masks.iter().any(|mask| {
            mask.enabled
                && mask.mode != crate::core::mask::MaskMode::None
                && (mask.expansion.evaluate(frame).abs() > 0.01 || mask.wiggle.is_some())
        }) {
            return Some("mask expansion/wiggle requires CPU renderer");
        }
        if matches!(
            layer.layer_type,
            crate::core::timeline::LayerType::AdjustmentLayer
                | crate::core::timeline::LayerType::Particle { .. }
                | crate::core::timeline::LayerType::Model3D { .. }
        ) {
            return Some("adjustment/particle/3D model layers require CPU renderer");
        }
        if matches!(
            &layer.layer_type,
            crate::core::timeline::LayerType::Shape {
                shape_type: crate::core::timeline::ShapeType::FreeformBezier { .. },
                ..
            }
        ) {
            return Some("freeform shapes require CPU renderer");
        }
        if matches!(
            &layer.layer_type,
            crate::core::timeline::LayerType::Shape {
                extrusion_depth,
                bevel_depth,
                ..
            } if *extrusion_depth > 0.0 || *bevel_depth > 0.0
        ) {
            return Some("extruded/beveled shapes require CPU renderer");
        }
        if matches!(
            &layer.layer_type,
            crate::core::timeline::LayerType::Shape { stroke_width, .. }
                if *stroke_width > 0.0
        ) {
            return Some("shape strokes require CPU renderer");
        }
        if matches!(
            layer.layer_type,
            crate::core::timeline::LayerType::Text { .. }
        ) && (layer
            .text_animator
            .as_ref()
            .is_some_and(|animator| animator.enabled)
            || layer
                .text_animator_stack
                .as_ref()
                .is_some_and(|stack| !stack.animators.is_empty()))
        {
            return Some("text animators require CPU renderer");
        }
        if dof_enabled && layer.is_3d {
            return Some("3D depth-of-field requires CPU renderer");
        }
        if !matches!(
            layer.blend_mode,
            crate::core::timeline::BlendMode::Normal | crate::core::timeline::BlendMode::Screen
        ) {
            return Some("unsupported blend mode requires CPU renderer");
        }
        if layer.style.drop_shadow.enabled
            || layer.style.outer_glow.enabled
            || layer.style.inner_glow.enabled
            || layer.style.satin.enabled
        {
            return Some("layer styles require CPU renderer");
        }
        if layer.motion_blur && comp.motion_blur_shutter_angle > 0.0 {
            return Some("motion blur requires CPU renderer");
        }
        if layer.effects_enabled {
            let mut last_order: Option<u32> = None;
            for effect in &layer.effects {
                if effect.enabled {
                    match crate::core::effect_plugin::gpu_preview_effect_slot(&effect.effect_type) {
                        Some(slot) => {
                            let order = slot.execution_order();
                            if slot == crate::core::effect_plugin::GpuEffectSlot::Posterize
                                && last_order.is_some()
                            {
                                return Some(
                                    "Posterize effect stacks require CPU renderer to preserve 8-bit stage rounding",
                                );
                            }
                            if let Some(prev) = last_order {
                                if order == prev {
                                    return Some(
                                        "competing effects in the same GPU shader slot require CPU renderer",
                                    );
                                } else if order < prev {
                                    return Some(
                                        "effect ordering differs from single-pass GPU pipeline order; CPU renderer required",
                                    );
                                }
                            }
                            last_order = Some(order);
                        }
                        None => {
                            return Some("unsupported effect requires CPU renderer");
                        }
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_preview_falls_back_when_lens_flare_stack_order_differs_from_shader() {
        use crate::core::{
            property::Animatable,
            timeline::{Effect, EffectType},
        };

        fn constant<T: Clone>(value: T) -> Animatable<T> {
            Animatable::new_constant(value)
        }
        let flare = Effect {
            id: "flare".to_owned(),
            name: "Lens Flare".to_owned(),
            effect_type: EffectType::LensFlare {
                enabled: constant(1.0),
                position_x: constant(0.5),
                position_y: constant(0.5),
                intensity: constant(1.0),
                threshold: constant(1.0),
                color: constant([1.0, 1.0, 1.0, 1.0]),
                link_to_light: None,
            },
            enabled: true,
        };
        let tint = Effect {
            id: "tint".to_owned(),
            name: "Color Tint".to_owned(),
            effect_type: EffectType::ColorTint {
                color: constant([0.8, 0.2, 0.1, 1.0]),
                intensity: constant(50.0),
            },
            enabled: true,
        };
        let mut composition = Composition::new(
            "gpu_order".to_owned(),
            "GPU order".to_owned(),
            64,
            64,
            30,
            10,
        );
        let mut layer = Layer::new(
            "solid".to_owned(),
            "Solid".to_owned(),
            LayerType::Solid {
                color: [0.2, 0.4, 0.6, 1.0],
            },
            10,
        );
        layer.effects = vec![flare.clone(), tint.clone()];
        composition.layers.push(layer);
        assert!(
            !software_preview_required(&composition, 0),
            "flare then tint matches the shader's sample-then-color pass order"
        );

        composition.layers[0].effects = vec![tint, flare];
        assert!(software_preview_required(&composition, 0));
        assert!(
            software_preview_reason(&composition, 0)
                .is_some_and(|reason| reason.contains("effect ordering"))
        );
    }

    #[test]
    fn gpu_preview_falls_back_when_posterize_follows_another_effect() {
        use crate::core::{
            property::Animatable,
            timeline::{Effect, EffectType},
        };

        let mut composition = Composition::new(
            "gpu_posterize_stack".to_owned(),
            "GPU Posterize Stack".to_owned(),
            64,
            64,
            30,
            1,
        );
        let mut layer = Layer::new(
            "solid".to_owned(),
            "Solid".to_owned(),
            LayerType::Solid {
                color: [0.2, 0.4, 0.6, 1.0],
            },
            1,
        );
        layer.effects = vec![
            Effect {
                id: "levels".to_owned(),
                name: "Levels".to_owned(),
                effect_type: EffectType::Levels {
                    input_black: Animatable::new_constant(0.03),
                    input_white: Animatable::new_constant(0.97),
                    gamma: Animatable::new_constant(1.15),
                    output_black: Animatable::new_constant(0.01),
                    output_white: Animatable::new_constant(0.99),
                },
                enabled: true,
            },
            Effect {
                id: "posterize".to_owned(),
                name: "Posterize".to_owned(),
                effect_type: EffectType::Posterize {
                    levels: Animatable::new_constant(6.0),
                },
                enabled: true,
            },
        ];
        composition.layers.push(layer);

        assert_eq!(
            software_preview_reason(&composition, 0),
            Some("Posterize effect stacks require CPU renderer to preserve 8-bit stage rounding")
        );
    }
}
