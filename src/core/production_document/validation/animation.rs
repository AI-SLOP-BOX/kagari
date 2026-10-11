use super::*;

pub(super) fn scalar_animation_is_finite(value: &crate::core::property::Animatable<f32>) -> bool {
    match value {
        crate::core::property::Animatable::Constant(value) => value.is_finite(),
        crate::core::property::Animatable::Animated(keyframes) => {
            keyframes_are_strictly_ordered(keyframes)
                && keyframes.iter().all(|keyframe| keyframe.value.is_finite())
        }
    }
}

pub(super) fn keyframes_are_strictly_ordered<T>(keyframes: &[Keyframe<T>]) -> bool {
    keyframes
        .windows(2)
        .all(|pair| pair[0].frame < pair[1].frame)
}

pub(super) fn scalar_animation_is_nonnegative(
    value: &crate::core::property::Animatable<f32>,
) -> bool {
    match value {
        crate::core::property::Animatable::Constant(value) => value.is_finite() && *value >= 0.0,
        crate::core::property::Animatable::Animated(keyframes) => {
            keyframes_are_strictly_ordered(keyframes)
                && keyframes
                    .iter()
                    .all(|keyframe| keyframe.value.is_finite() && keyframe.value >= 0.0)
        }
    }
}

pub(super) fn scalar_animation_is_percentage(
    value: &crate::core::property::Animatable<f32>,
) -> bool {
    match value {
        crate::core::property::Animatable::Constant(value) => {
            value.is_finite() && (0.0..=100.0).contains(value)
        }
        crate::core::property::Animatable::Animated(keyframes) => {
            keyframes_are_strictly_ordered(keyframes)
                && keyframes.iter().all(|keyframe| {
                    keyframe.value.is_finite() && (0.0..=100.0).contains(&keyframe.value)
                })
        }
    }
}

pub(super) fn scalar_animation_is_at_least(
    value: &crate::core::property::Animatable<f32>,
    minimum: f32,
) -> bool {
    match value {
        crate::core::property::Animatable::Constant(value) => {
            value.is_finite() && *value >= minimum
        }
        crate::core::property::Animatable::Animated(keyframes) => {
            keyframes_are_strictly_ordered(keyframes)
                && keyframes
                    .iter()
                    .all(|keyframe| keyframe.value.is_finite() && keyframe.value >= minimum)
        }
    }
}

pub(super) fn scalar_animation_is_at_most(
    value: &crate::core::property::Animatable<f32>,
    maximum: f32,
) -> bool {
    match value {
        crate::core::property::Animatable::Constant(value) => {
            value.is_finite() && *value <= maximum
        }
        crate::core::property::Animatable::Animated(keyframes) => {
            keyframes_are_strictly_ordered(keyframes)
                && keyframes
                    .iter()
                    .all(|keyframe| keyframe.value.is_finite() && keyframe.value <= maximum)
        }
    }
}

pub(crate) fn valid_gradient_stops(colors: &[[f32; 4]], stops: &[f32]) -> bool {
    colors.len() >= 2
        && colors.len() == stops.len()
        && colors
            .iter()
            .all(|color| color.iter().all(|value| value.is_finite()))
        && stops.windows(2).all(|pair| {
            pair[0].is_finite()
                && pair[1].is_finite()
                && (0.0..=1.0).contains(&pair[0])
                && pair[0] <= pair[1]
        })
        && stops
            .first()
            .is_some_and(|stop| stop.is_finite() && (0.0..=1.0).contains(stop))
        && stops
            .last()
            .is_some_and(|stop| stop.is_finite() && (0.0..=1.0).contains(stop))
}

pub(crate) fn effect_debug_is_finite(debug: &str) -> bool {
    !debug
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '-')
        .any(|token| matches!(token, "NaN" | "inf" | "-inf"))
}

pub(crate) fn particle_emitter_is_valid(
    emitter: &crate::core::particle_system::ParticleEmitter,
) -> bool {
    let finite = [
        emitter.rate,
        emitter.lifetime,
        emitter.lifetime_variance,
        emitter.speed,
        emitter.speed_variance,
        emitter.spread_degrees,
        emitter.emitter_size[0],
        emitter.emitter_size[1],
        emitter.gravity[0],
        emitter.gravity[1],
        emitter.wind[0],
        emitter.wind[1],
        emitter.turbulence,
        emitter.color_start[0],
        emitter.color_start[1],
        emitter.color_start[2],
        emitter.color_start[3],
        emitter.color_end[0],
        emitter.color_end[1],
        emitter.color_end[2],
        emitter.color_end[3],
        emitter.size_start,
        emitter.size_end,
        emitter.opacity_start,
        emitter.opacity_end,
        emitter.rotation_speed,
        emitter.rotation_start,
        emitter.rotation_speed_variance,
        emitter.wind_gust_strength,
        emitter.wind_gust_frequency,
        emitter.drag,
        emitter.restitution,
        emitter.surface_friction,
        emitter.particle_diameter,
        emitter.trail_taper,
        emitter.vortex_strength,
        emitter.vortex_center[0],
        emitter.vortex_center[1],
        emitter.attract_strength,
        emitter.attract_center[0],
        emitter.attract_center[1],
        emitter.depth_range[0],
        emitter.depth_range[1],
        emitter.death_spawn_speed_scale,
        emitter.death_spawn_life_scale,
    ];
    finite.iter().all(|value| value.is_finite())
        && emitter.rate >= 0.0
        && emitter.max_particles > 0
        && emitter.max_particles <= 10_000_000
        && emitter.lifetime > 0.0
        && emitter.lifetime_variance >= 0.0
        && emitter.speed >= 0.0
        && emitter.speed_variance >= 0.0
        && (0.0..=360.0).contains(&emitter.spread_degrees)
        && emitter.emitter_size.iter().all(|value| *value >= 0.0)
        && (0.0..=1.0).contains(&emitter.color_start[3])
        && (0.0..=1.0).contains(&emitter.color_end[3])
        && emitter.size_start >= 0.0
        && emitter.size_end >= 0.0
        && emitter.opacity_start >= 0.0
        && emitter.opacity_start <= 1.0
        && emitter.opacity_end >= 0.0
        && emitter.opacity_end <= 1.0
        && emitter.drag >= 0.0
        && emitter.restitution >= 0.0
        && emitter.surface_friction >= 0.0
        && emitter.particle_diameter > 0.0
        && emitter.depth_range[0] <= emitter.depth_range[1]
        && emitter.death_spawn_speed_scale >= 0.0
        && emitter.death_spawn_life_scale >= 0.0
        && emitter.blend_mode <= 2
        && emitter
            .gravity_curve
            .0
            .iter()
            .all(|value| value.is_finite())
        && (!emitter.collision_enabled
            || (emitter
                .collision_bounds
                .iter()
                .all(|value| value.is_finite())
                && emitter.collision_bounds[0] <= emitter.collision_bounds[2]
                && emitter.collision_bounds[1] <= emitter.collision_bounds[3]))
}

pub(crate) fn light_type_is_valid(light_type: &crate::core::timeline::LightType) -> bool {
    match light_type {
        crate::core::timeline::LightType::Spot {
            cone_angle_deg,
            cone_feather_pct,
        } => {
            cone_angle_deg.is_finite()
                && (0.0..=180.0).contains(cone_angle_deg)
                && cone_feather_pct.is_finite()
                && (0.0..=100.0).contains(cone_feather_pct)
        }
        _ => true,
    }
}

pub(super) fn vector_animation_is_finite(
    value: &crate::core::property::Animatable<[f32; 2]>,
) -> bool {
    match value {
        crate::core::property::Animatable::Constant(value) => {
            value.iter().all(|component| component.is_finite())
        }
        crate::core::property::Animatable::Animated(keyframes) => {
            keyframes_are_strictly_ordered(keyframes)
                && keyframes
                    .iter()
                    .all(|keyframe| keyframe.value.iter().all(|component| component.is_finite()))
        }
    }
}

pub(super) fn vector3_animation_is_finite(
    value: &crate::core::property::Animatable<[f32; 3]>,
) -> bool {
    match value {
        crate::core::property::Animatable::Constant(value) => {
            value.iter().all(|component| component.is_finite())
        }
        crate::core::property::Animatable::Animated(keyframes) => {
            keyframes_are_strictly_ordered(keyframes)
                && keyframes
                    .iter()
                    .all(|keyframe| keyframe.value.iter().all(|component| component.is_finite()))
        }
    }
}
