//! Shared render capability checks (GPU vs CPU fallback).

use crate::core::timeline::Composition;

/// Check if a composition requires CPU (software) rendering at a given frame.
/// Returns true if the composition uses any features unsupported by the GPU renderer.
pub fn software_preview_required(comp: &Composition, frame: u32) -> bool {
    software_preview_reason(comp, frame).is_some()
}

/// Explains why software preview is required at a given frame, or None if GPU is capable.
/// All compositions now execute on the GPU compositor (`WgpuRenderer`); layers using
/// features outside the single-pass WGSL shader are rasterized into GPU textures per layer.
pub fn software_preview_reason(_comp: &Composition, _frame: u32) -> Option<&'static str> {
    None
}

/// Returns true when `layer` requires per-layer texture rasterization before GPU compositing
/// (e.g., particles, 3D models, layer styles, puppet pins, or multi-pass effect chains).
pub(crate) fn layer_needs_raster_fallback(
    comp: &Composition,
    layer: &crate::core::timeline::Layer,
    frame: u32,
) -> bool {
    layer_raster_fallback_reason(comp, layer, frame).is_some()
}

pub(crate) fn layer_gpu_motion_blur_params(
    comp: &Composition,
    layer: &crate::core::timeline::Layer,
    frame: u32,
) -> Option<([f32; 2], u32, f32)> {
    if !(layer.motion_blur && comp.motion_blur_shutter_angle > 0.0) {
        return None;
    }
    let (p_prev, _, _, _) = comp.resolve_world_transform(layer, frame.saturating_sub(1));
    let (p_next, _, _, _) = comp.resolve_world_transform(layer, frame.saturating_add(1));
    let vel_x = (p_next[0] - p_prev[0]) * 0.5;
    let vel_y = (p_next[1] - p_prev[1]) * 0.5;
    let speed = (vel_x * vel_x + vel_y * vel_y).sqrt();
    if speed <= 0.05 {
        return None;
    }
    let shutter = (comp.motion_blur_shutter_angle / 360.0).clamp(0.0, 2.0);
    let samples = ((speed * shutter).ceil() as u32).clamp(2, 32);
    let phase_offset = (comp.motion_blur_shutter_phase + 90.0) / 360.0;
    let fps_scale = comp.fps.max(1) as f32 / 24.0;
    Some(([vel_x * shutter * fps_scale, vel_y * shutter * fps_scale], samples, phase_offset))
}

pub(crate) fn layer_needs_gpu_multipass_effects(layer: &crate::core::timeline::Layer) -> bool {
    if !layer.effects_enabled {
        return false;
    }
    let mut last_order: Option<u32> = None;
    let mut needs_multipass = false;
    for effect in &layer.effects {
        if !effect.enabled {
            continue;
        }
        if !crate::core::effect_plugin::adapter::has_wgsl_shader_effect(&effect.effect_type) {
            return false;
        }
        if matches!(
            effect.effect_type,
            crate::core::timeline::EffectType::Echo { .. }
                | crate::core::timeline::EffectType::SetMatte { .. }
                | crate::core::timeline::EffectType::SliderControl { .. }
                | crate::core::timeline::EffectType::AngleControl { .. }
                | crate::core::timeline::EffectType::PointControl { .. }
                | crate::core::timeline::EffectType::ColorControl { .. }
                | crate::core::timeline::EffectType::CheckboxControl { .. }
                | crate::core::timeline::EffectType::DropdownControl { .. }
                | crate::core::timeline::EffectType::Point3DControl { .. }
                | crate::core::timeline::EffectType::BassTreble { .. }
                | crate::core::timeline::EffectType::Flanger { .. }
                | crate::core::timeline::EffectType::Chorus { .. }
                | crate::core::timeline::EffectType::ParametricEQ { .. }
                | crate::core::timeline::EffectType::MergePaths { .. }
                | crate::core::timeline::EffectType::OffsetPath { .. }
        ) {
            continue;
        }
        let Some(slot) = crate::core::effect_plugin::gpu_preview_effect_slot(&effect.effect_type)
        else {
            // Extended WGSL effect (e.g. GaussianBlur, Glow, HueSaturation, TurbulentDisplace):
            // always run in its own clean scratch pass so parameters never collide.
            needs_multipass = true;
            continue;
        };
        let order = slot.execution_order();
        if (slot == crate::core::effect_plugin::GpuEffectSlot::Posterize && last_order.is_some())
            || last_order.is_some_and(|prev| order <= prev)
        {
            needs_multipass = true;
        }
        last_order = Some(order);
    }
    needs_multipass
}

fn layer_raster_fallback_reason(
    _comp: &Composition,
    layer: &crate::core::timeline::Layer,
    _frame: u32,
) -> Option<&'static str> {
        if !matches!(
            layer.blend_mode,
            crate::core::timeline::BlendMode::Normal
                | crate::core::timeline::BlendMode::Multiply
                | crate::core::timeline::BlendMode::Screen
                | crate::core::timeline::BlendMode::Add
                | crate::core::timeline::BlendMode::LinearDodge
                | crate::core::timeline::BlendMode::Behind
                | crate::core::timeline::BlendMode::Overlay
                | crate::core::timeline::BlendMode::Darken
                | crate::core::timeline::BlendMode::Lighten
                | crate::core::timeline::BlendMode::SoftLight
                | crate::core::timeline::BlendMode::HardLight
                | crate::core::timeline::BlendMode::Difference
                | crate::core::timeline::BlendMode::Exclusion
                | crate::core::timeline::BlendMode::Divide
                | crate::core::timeline::BlendMode::Subtract
                | crate::core::timeline::BlendMode::ColorBurn
                | crate::core::timeline::BlendMode::LinearBurn
                | crate::core::timeline::BlendMode::VividLight
                | crate::core::timeline::BlendMode::ColorDodge
                | crate::core::timeline::BlendMode::LinearLight
                | crate::core::timeline::BlendMode::Color
                | crate::core::timeline::BlendMode::Hue
                | crate::core::timeline::BlendMode::Saturation
                | crate::core::timeline::BlendMode::Luminosity
                | crate::core::timeline::BlendMode::StencilAlpha
                | crate::core::timeline::BlendMode::StencilLuma
                | crate::core::timeline::BlendMode::SilhouetteAlpha
                | crate::core::timeline::BlendMode::SilhouetteLuma
                | crate::core::timeline::BlendMode::AlphaAdd
        ) {
            return Some("unsupported blend mode requires CPU renderer");
        }
        if layer.effects_enabled {
            for effect in &layer.effects {
                if effect.enabled
                    && !crate::core::effect_plugin::adapter::has_wgsl_shader_effect(&effect.effect_type)
                {
                    return Some("unsupported effect requires CPU renderer");
                }
            }
        }
        None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::timeline::{Layer, LayerType};

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
            !layer_needs_raster_fallback(&composition, &composition.layers[0], 0),
            "flare then tint matches the shader's sample-then-color pass order"
        );

        composition.layers[0].effects = vec![tint, flare];
        assert!(!software_preview_required(&composition, 0));
        assert!(!layer_needs_raster_fallback(
            &composition,
            &composition.layers[0],
            0
        ));
        assert!(layer_needs_gpu_multipass_effects(&composition.layers[0]));
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

        assert!(!software_preview_required(&composition, 0));
        assert!(!layer_needs_raster_fallback(
            &composition,
            &composition.layers[0],
            0
        ));
        assert!(layer_needs_gpu_multipass_effects(&composition.layers[0]));
    }

    #[test]
    fn gpu_preview_supports_add_and_linear_dodge_blend_modes() {
        use crate::core::timeline::{BlendMode, Layer, LayerType};

        let mut composition =
            Composition::new("gpu_add".to_owned(), "GPU Add".to_owned(), 64, 64, 30, 1);
        let mut layer = Layer::new(
            "solid".to_owned(),
            "Solid".to_owned(),
            LayerType::Solid {
                color: [0.3, 0.5, 0.7, 1.0],
            },
            1,
        );
        layer.blend_mode = BlendMode::Add;
        composition.layers.push(layer);
        assert!(!software_preview_required(&composition, 0));

        composition.layers[0].blend_mode = BlendMode::LinearDodge;
        assert!(!software_preview_required(&composition, 0));

        composition.layers[0].blend_mode = BlendMode::Behind;
        assert!(!software_preview_required(&composition, 0));

        composition.layers[0].blend_mode = BlendMode::Multiply;
        assert!(!software_preview_required(&composition, 0));

        composition.background_color[3] = 0.0;
        assert!(!software_preview_required(&composition, 0));
        assert!(!layer_needs_raster_fallback(
            &composition,
            &composition.layers[0],
            0
        ));

        composition.background_color[3] = 1.0;
        composition.layers[0].blend_mode = BlendMode::Normal;
        composition.dither_output = true;
        assert!(!software_preview_required(&composition, 0));

        composition.dither_output = false;
        composition.layers[0].layer_type = LayerType::Shape {
            shape_type: crate::core::timeline::ShapeType::Rectangle {
                width: crate::core::property::Animatable::new_constant(120.0),
                height: crate::core::property::Animatable::new_constant(80.0),
                corner_radius: crate::core::property::Animatable::new_constant(12.0),
            },
            color: [1.0, 0.2, 0.2, 1.0],
            stroke_color: [1.0, 1.0, 1.0, 1.0],
            stroke_width: 4.0,
            fill_type: crate::core::timeline::ShapeFillType::Solid,
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        };
        assert!(!software_preview_required(&composition, 0));

        let mut mask = crate::core::mask::Mask::new_rect(
            "m1".to_owned(),
            "Mask 1".to_owned(),
            8.0,
            8.0,
            40.0,
            40.0,
        );
        mask.expansion = crate::core::property::Animatable::new_constant(4.0);
        composition.layers[0].masks.push(mask);
        assert!(!software_preview_required(&composition, 0));

        composition.working_color_space =
            crate::core::color_science::WorkingColorSpace::AcesCG;
        assert!(!software_preview_required(&composition, 0));

        composition.working_color_space =
            crate::core::color_science::WorkingColorSpace::DisplayP3;
        assert!(!software_preview_required(&composition, 0));
    }
}
