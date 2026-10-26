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
                || (layer.motion_blur && comp.motion_blur_shutter_angle > 0.0)
                || (layer.effects_enabled && {
                    let mut last_order: Option<u32> = None;
                    let mut has_conflict_or_unsupported = false;
                    for effect in &layer.effects {
                        if effect.enabled {
                            match crate::core::effect_plugin::gpu_preview_effect_slot(&effect.effect_type) {
                                Some(slot) => {
                                    let order = slot.execution_order();
                                    // Must be strictly increasing: no duplicate slots and must match shader execution order
                                    if let Some(prev) = last_order {
                                        if order <= prev {
                                            has_conflict_or_unsupported = true;
                                            break;
                                        }
                                    }
                                    last_order = Some(order);
                                }
                                None => {
                                    has_conflict_or_unsupported = true;
                                    break;
                                }
                            }
                        }
                    }
                    has_conflict_or_unsupported
                })
        })
}

/// Explains why software preview is required at a given frame, or None if GPU is capable.
pub fn software_preview_reason(comp: &Composition, frame: u32) -> Option<&'static str> {
    let dof_enabled = comp.resolve_camera().dof_enabled_at(frame);

    for layer in comp.layers.iter().filter(|l| l.is_active(frame)) {
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
        if dof_enabled && layer.is_3d {
            return Some("3D depth-of-field requires CPU renderer");
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
                            if let Some(prev) = last_order {
                                if order == prev {
                                    return Some("competing effects in the same GPU shader slot require CPU renderer");
                                } else if order < prev {
                                    return Some("effect ordering differs from single-pass GPU pipeline order; CPU renderer required");
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

