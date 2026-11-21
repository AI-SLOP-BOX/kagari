//! Keyframe baking and motion design spring overshoot expressions.

use super::world::PhysicsWorld;
use crate::core::keyframe::{InterpolationType, Keyframe};
use std::collections::HashMap;

pub type PhysicsLayerKeyframes = (Vec<Keyframe<[f32; 2]>>, Vec<Keyframe<f32>>);
pub type PhysicsBakeResult = HashMap<usize, PhysicsLayerKeyframes>;

/// Bakes physics simulation trajectories directly into timeline keyframes for mapped layers.
pub fn bake_physics_simulation_to_keyframes(
    world: &mut PhysicsWorld,
    start_frame: u32,
    end_frame: u32,
    fps: f32,
) -> PhysicsBakeResult {
    let mut results: PhysicsBakeResult = HashMap::new();

    let dt = 1.0 / fps.max(1.0);

    for frame in start_frame..=end_frame {
        for body in &world.bodies {
            if let Some(layer_id) = body.layer_id {
                let entry = results
                    .entry(layer_id)
                    .or_insert_with(|| (Vec::new(), Vec::new()));

                entry.0.push(Keyframe::new(
                    frame,
                    body.position,
                    InterpolationType::Linear,
                ));
                entry.1.push(Keyframe::new(
                    frame,
                    body.rotation_deg,
                    InterpolationType::Linear,
                ));
            }
        }

        world.step(dt);
    }

    results
}

/// Evaluates classic After Effects spring overshoot (e.g. bounce, inertia expressions).
/// `freq`: Oscillations per second (default ~3.0)
/// `decay`: Damping decay rate (default ~5.0)
/// `amplitude`: Overshoot scale percentage (default 0.1)
pub fn calc_spring_overshoot(t: f32, freq: f32, decay: f32, amplitude: f32) -> f32 {
    if t <= 0.0 {
        return 0.0;
    }
    let w = freq * std::f32::consts::TAU;
    amplitude * (-decay * t).exp() * (w * t).sin()
}

/// Evaluates bouncing ball decay formula for floor impacts.
pub fn calc_bounce_decay(t: f32, bounces: u32, elasticity: f32) -> f32 {
    if t <= 0.0 {
        return 1.0;
    }
    let bounce_time = 1.0 / (bounces.max(1) as f32);
    let current_bounce = (t / bounce_time).floor() as u32;
    if current_bounce >= bounces {
        return 0.0;
    }
    let local_t = (t % bounce_time) / bounce_time;
    let amplitude = elasticity.powi(current_bounce as i32);
    let height = 4.0 * local_t * (1.0 - local_t);
    height * amplitude
}
