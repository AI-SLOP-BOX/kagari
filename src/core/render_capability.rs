//! Shared render capability checks (GPU vs CPU fallback).

use crate::core::timeline::Composition;

/// Check if a composition requires CPU (software) rendering at a given frame.
/// Returns true if the composition uses any features unsupported by the GPU renderer.
pub fn software_preview_required(comp: &Composition, frame: u32) -> bool {
    let dof_enabled = comp.resolve_camera().dof_enabled_at(frame);

    comp.layers
        .iter()
        .filter(|layer| layer.is_active(frame))
        .any(|layer| {
            !layer.puppet_pins.is_empty()
                || layer.masks.iter().any(|mask| {
                    mask.enabled
                        && mask.mode != crate::core::mask::MaskMode::None
                        && (mask.expansion.evaluate(frame).abs() > 0.01 || mask.wiggle.is_some())
                })
                || matches!(
                    layer.layer_type,
                    crate::core::timeline::LayerType::AdjustmentLayer
                        | crate::core::timeline::LayerType::Particle { .. }
                        | crate::core::timeline::LayerType::Model3D { .. }
                )
                || matches!(
                    &layer.layer_type,
                    crate::core::timeline::LayerType::Shape {
                        shape_type: crate::core::timeline::ShapeType::FreeformBezier { .. },
                        ..
                    }
                )
                || matches!(
                    &layer.layer_type,
                    crate::core::timeline::LayerType::Shape {
                        extrusion_depth,
                        bevel_depth,
                        ..
                    } if *extrusion_depth > 0.0 || *bevel_depth > 0.0
                )
                || matches!(
                    &layer.layer_type,
                    crate::core::timeline::LayerType::Shape { stroke_width, .. }
                        if *stroke_width > 0.0
                )
                || (matches!(
                    layer.layer_type,
                    crate::core::timeline::LayerType::Text { .. }
                ) && (layer
                    .text_animator
                    .as_ref()
                    .is_some_and(|animator| animator.enabled)
                    || layer
                        .text_animator_stack
                        .as_ref()
                        .is_some_and(|stack| !stack.animators.is_empty())))
                || (dof_enabled && layer.is_3d)
                || !matches!(
                    layer.blend_mode,
                    crate::core::timeline::BlendMode::Normal
                        | crate::core::timeline::BlendMode::Multiply
                        | crate::core::timeline::BlendMode::Screen
                        | crate::core::timeline::BlendMode::Overlay
                        | crate::core::timeline::BlendMode::Add
                        | crate::core::timeline::BlendMode::Darken
                        | crate::core::timeline::BlendMode::Lighten
                )
                || layer.style.drop_shadow.enabled
                || layer.style.outer_glow.enabled
                || layer.style.inner_glow.enabled
                || layer.style.satin.enabled
                || (layer.effects_enabled && {
                    let mut enabled_effects = 0usize;
                    let mut has_unsupported_effect = false;
                    for effect in &layer.effects {
                        if effect.enabled {
                            enabled_effects += 1;
                            has_unsupported_effect |=
                                !crate::core::effect_plugin::gpu_preview_effect_supported(
                                    &effect.effect_type,
                                );
                        }
                    }
                    enabled_effects > 1 || has_unsupported_effect
                })
        })
}
