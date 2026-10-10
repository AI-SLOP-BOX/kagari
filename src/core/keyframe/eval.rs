use super::types::{BezierControlPoint, InterpolationType, Keyframe};

/// Evaluates a cubic bezier curve at x using Newton-Raphson or binary search.
/// Control points are (0,0), (x1, y1), (x2, y2), (1,1).
pub fn solve_bezier_eased_time(x: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }

    // Coefficients of the cubic bezier equation for X(t) = x
    // X(t) = 3*(1-t)^2*t*x1 + 3*(1-t)*t^2*x2 + t^3
    //      = t^3 * (1 - 3*x2 + 3*x1) + t^2 * (3*x2 - 6*x1) + t * (3*x1)
    let c = 3.0 * x1;
    let b = 3.0 * (x2 - x1) - c;
    let a = 1.0 - 3.0 * x2 + c;

    // Helper functions
    let sample_curve_x = |t: f32| ((a * t + b) * t + c) * t;
    let sample_curve_derivative_x = |t: f32| (3.0 * a * t + 2.0 * b) * t + c;

    // Newton-Raphson iteration (12 steps, 1e-7 epsilon tolerance for sub-pixel accuracy)
    let mut t_guess = x;
    for _ in 0..12 {
        let x_guess = sample_curve_x(t_guess) - x;
        if x_guess.abs() < 1e-7 {
            break;
        }
        let d = sample_curve_derivative_x(t_guess);
        if d.abs() < 1e-7 {
            break;
        }
        // Clamp to [0, 1] to prevent divergence on extreme easing curves (influence ~100%)
        t_guess = (t_guess - x_guess / d).clamp(0.0, 1.0);
    }

    // Fallback to high-precision binary search (24 steps) if Newton-Raphson did not converge
    if (sample_curve_x(t_guess) - x).abs() > 1e-6 {
        let mut t_lower = 0.0;
        let mut t_upper = 1.0;
        t_guess = x;
        for _ in 0..24 {
            let x_guess = sample_curve_x(t_guess);
            if (x_guess - x).abs() < 1e-6 {
                break;
            }
            if x < x_guess {
                t_upper = t_guess;
            } else {
                t_lower = t_guess;
            }
            t_guess = (t_lower + t_upper) * 0.5;
        }
    }

    // Compute Y(t)
    // Y(t) = 3*(1-t)^2*t*y1 + 3*(1-t)*t^2*y2 + t^3
    let cy = 3.0 * y1;
    let by = 3.0 * (y2 - y1) - cy;
    let ay = 1.0 - 3.0 * y2 + cy;

    ((ay * t_guess + by) * t_guess + cy) * t_guess
}

/// Compute cubic Bezier control points (x1, y1, x2, y2) from AE keyframe speed and influence parameters.
pub fn compute_ae_bezier_control_points(
    outgoing: &BezierControlPoint,
    incoming: &BezierControlPoint,
    delta_frame: f32,
    delta_val: f32,
    fps: f32,
) -> [f32; 4] {
    let dt = (delta_frame / fps.max(1.0)).max(0.001);
    let x1 = outgoing.influence.clamp(0.0, 1.0);
    let x2 = 1.0 - incoming.influence.clamp(0.0, 1.0);

    let y1 = if delta_val.abs() > 1e-6 {
        (outgoing.speed * dt * x1) / delta_val
    } else {
        0.0
    };
    let y2 = if delta_val.abs() > 1e-6 {
        1.0 - (incoming.speed * dt * (1.0 - x2)) / delta_val
    } else {
        1.0
    };

    [x1, y1.clamp(-2.0, 3.0), x2, y2.clamp(-2.0, 3.0)]
}

/// Inverse mapping: reconstruct AE Speed and Influence from cubic Bezier control points [x1, y1, x2, y2].
pub fn ae_bezier_control_points_to_speed_influence(
    ctrl_pts: [f32; 4],
    delta_frame: f32,
    delta_val: f32,
    fps: f32,
) -> (BezierControlPoint, BezierControlPoint) {
    let dt = (delta_frame / fps.max(1.0)).max(0.001);
    let [x1, y1, x2, y2] = ctrl_pts;

    let inf_out = x1.clamp(0.01, 1.0);
    let inf_in = (1.0 - x2).clamp(0.01, 1.0);

    let speed_out = if inf_out > 0.001 && dt > 0.001 {
        (y1 * delta_val) / (dt * inf_out)
    } else {
        0.0
    };

    let speed_in = if inf_in > 0.001 && dt > 0.001 {
        ((1.0 - y2) * delta_val) / (dt * inf_in)
    } else {
        0.0
    };

    (
        BezierControlPoint {
            influence: inf_out,
            speed: speed_out,
        },
        BezierControlPoint {
            influence: inf_in,
            speed: speed_in,
        },
    )
}

/// Evaluate instantaneous velocity (dv/dt in units per second) for a scalar keyframe sequence.
pub fn evaluate_scalar_speed_at_frame(keyframes: &[Keyframe<f32>], frame: f32, fps: u32) -> f32 {
    if keyframes.len() < 2 {
        return 0.0;
    }
    let fps_f = fps.max(1) as f32;
    let eps_frame = 0.01f32;

    let f_prev = (frame - eps_frame).max(0.0);
    let f_next = frame + eps_frame;
    let dt = (f_next - f_prev) / fps_f;
    if dt <= 0.0001 {
        return 0.0;
    }

    let v_prev = evaluate_scalar_keyframes(keyframes, f_prev, fps);
    let v_next = evaluate_scalar_keyframes(keyframes, f_next, fps);

    (v_next - v_prev) / dt
}

/// Evaluate a scalar keyframe sequence at a fractional frame.
pub fn evaluate_scalar_keyframes(keyframes: &[Keyframe<f32>], frame: f32, _fps: u32) -> f32 {
    if keyframes.is_empty() {
        return 0.0;
    }
    if keyframes.len() == 1 || frame <= keyframes[0].frame as f32 {
        return keyframes[0].value;
    }
    if frame >= keyframes.last().unwrap().frame as f32 {
        return keyframes.last().unwrap().value;
    }

    // Find bounding keyframe interval
    for i in 0..keyframes.len() - 1 {
        let k0 = &keyframes[i];
        let k1 = &keyframes[i + 1];
        let f0 = k0.frame as f32;
        let f1 = k1.frame as f32;

        if frame >= f0 && frame <= f1 {
            let span = (f1 - f0).max(0.001);
            let linear_t = (frame - f0) / span;

            return match &k0.interpolation {
                InterpolationType::Hold => k0.value,
                InterpolationType::Linear => k0.value + linear_t * (k1.value - k0.value),
                InterpolationType::Bezier {
                    outgoing,
                    incoming,
                    custom_bezier,
                } => {
                    let pts = custom_bezier.unwrap_or_else(|| {
                        compute_ae_bezier_control_points(
                            outgoing,
                            incoming,
                            span,
                            k1.value - k0.value,
                            _fps as f32,
                        )
                    });
                    let eased_t = solve_bezier_eased_time(linear_t, pts[0], pts[1], pts[2], pts[3]);
                    k0.value + eased_t * (k1.value - k0.value)
                }
            };
        }
    }

    keyframes.last().unwrap().value
}
