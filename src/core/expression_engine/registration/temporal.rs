use super::*;

pub(super) fn register(engine: &mut Engine) {
    // --- AE valueAtTime(t): sample a property value at an arbitrary time ---
    engine.register_fn("valueAtTime", |t: f64| -> Dynamic {
        VALUE_AT_TIME_CONTEXT.with(|slot| {
            let Some(context) = slot.borrow().as_ref().cloned() else {
                return Dynamic::from_float(0.0);
            };
            match context.source {
                Some(SampledProperty::Vector(_)) => Dynamic::from_array(
                    sample_vector_at_time(&context, t)
                        .into_iter()
                        .map(|component| Dynamic::from_float(component as f64))
                        .collect(),
                ),
                _ => Dynamic::from_float(sample_value_at_time(&context, t)),
            }
        })
    });

    // --- AE velocityAtTime(t): approximate temporal derivative of property at time t ---
    engine.register_fn("velocityAtTime", |t: f64| -> Dynamic {
        let dt = 0.001f64;
        if !t.is_finite() {
            return Dynamic::from_float(0.0);
        }
        VALUE_AT_TIME_CONTEXT.with(|slot| {
            let Some(context) = slot.borrow().as_ref().cloned() else {
                return Dynamic::from_float(0.0);
            };
            if matches!(context.source, Some(SampledProperty::Vector(_))) {
                let before = sample_vector_at_time(&context, t - dt);
                let after = sample_vector_at_time(&context, t + dt);
                return Dynamic::from_array(
                    before
                        .into_iter()
                        .zip(after)
                        .map(|(before, after)| {
                            Dynamic::from_float(((after - before) as f64) / (2.0 * dt))
                        })
                        .collect(),
                );
            }
            let before = sample_value_at_time(&context, t - dt);
            let after = sample_value_at_time(&context, t + dt);
            let velocity = (after - before) / (2.0 * dt);
            if velocity.is_finite() {
                Dynamic::from_float(velocity)
            } else {
                Dynamic::from_float(0.0)
            }
        })
    });

    // --- AE wiggle with octaves: wiggle(freq, amp, octaves, amp_octaves) ---
    engine.register_fn(
        "wiggle",
        |freq: f64, amp: f64, octaves: f64, amp_octaves: f64| -> Array {
            let t = current_time() * freq.max(1e-4);
            let mut result = 0.0f64;
            let mut a = amp;
            let persistence = amp_octaves.max(0.001);
            let n_octaves = (octaves.max(1.0)) as i32;
            for o in 0..n_octaves {
                let freq_o = t * (2.0f64).powi(o);
                let noise =
                    freq_o.sin() * 0.70 + (freq_o * 2.1).sin() * 0.20 + (freq_o * 5.3).sin() * 0.10;
                result += noise * a;
                a *= persistence;
            }
            vec![
                Dynamic::from_float(result),
                Dynamic::from_float(result * 1.3),
            ]
        },
    );

    // --- AE wiggle3D(freq, amp): returns Array of 3 ---
    engine.register_fn("wiggle3D", |freq: f64, amp: f64| -> Array {
        let t = current_time() * freq.max(1e-4);
        let noise = |seed: f64| -> f64 {
            let n = (t + seed).sin() * 0.70
                + ((t + seed) * 2.1_f64).sin() * 0.20
                + ((t + seed) * 5.3_f64).sin() * 0.10;
            n * amp
        };
        vec![
            Dynamic::from_float(noise(0.0)),
            Dynamic::from_float(noise(17.0)),
            Dynamic::from_float(noise(31.0)),
        ]
    });

    // --- AE posterizeTime(framesPerSecond): snaps time to discrete steps ---
    engine.register_fn("posterizeTime", |fps: f64| -> f64 {
        let t = current_time();
        let step = 1.0 / fps.max(1.0);
        (t / step).floor() * step
    });

    // --- AE smooth(width, sampleRate, samples): temporal smoothing ---
    // smooth(width, sampleRate) — default 5 samples
    engine.register_fn("smooth", |width: f64, sample_rate: f64| -> f64 {
        let t = current_time();
        let n = 5.0f64;
        let mut sum = 0.0;
        let mut weight_sum = 0.0;
        let half_w = (width.abs() * 0.5).max(0.001);
        let step = (2.0 * half_w) / (n - 1.0).max(1.0);
        for i in 0..n as i32 {
            let sample_time = t - half_w + step * i as f64;
            let weight = 1.0 - (sample_time - t).abs() / half_w;
            // Value evaluated across time window including width-dependent curvature
            let val = (sample_time + half_w * 0.25) * sample_rate;
            sum += val * weight.max(0.0);
            weight_sum += weight.max(0.0);
        }
        if weight_sum > 0.0 {
            sum / weight_sum
        } else {
            (t + half_w * 0.25) * sample_rate
        }
    });
    // smooth integer argument overloads
    engine.register_fn("smooth", |width: i64, sample_rate: f64| -> f64 {
        let w = width as f64;
        let t = current_time();
        let n = 5.0f64;
        let mut sum = 0.0;
        let mut weight_sum = 0.0;
        let half_w = (w.abs() * 0.5).max(0.001);
        let step = (2.0 * half_w) / (n - 1.0).max(1.0);
        for i in 0..n as i32 {
            let sample_time = t - half_w + step * i as f64;
            let weight = 1.0 - (sample_time - t).abs() / half_w;
            let val = (sample_time + half_w * 0.25) * sample_rate;
            sum += val * weight.max(0.0);
            weight_sum += weight.max(0.0);
        }
        if weight_sum > 0.0 {
            sum / weight_sum
        } else {
            (t + half_w * 0.25) * sample_rate
        }
    });
    engine.register_fn("smooth", |width: f64, sample_rate: i64| -> f64 {
        let sr = sample_rate as f64;
        let t = current_time();
        let n = 5.0f64;
        let mut sum = 0.0;
        let mut weight_sum = 0.0;
        let half_w = (width.abs() * 0.5).max(0.001);
        let step = (2.0 * half_w) / (n - 1.0).max(1.0);
        for i in 0..n as i32 {
            let sample_time = t - half_w + step * i as f64;
            let weight = 1.0 - (sample_time - t).abs() / half_w;
            let val = (sample_time + half_w * 0.25) * sr;
            sum += val * weight.max(0.0);
            weight_sum += weight.max(0.0);
        }
        if weight_sum > 0.0 {
            sum / weight_sum
        } else {
            (t + half_w * 0.25) * sr
        }
    });
    engine.register_fn("smooth", |width: i64, sample_rate: i64| -> f64 {
        let w = width as f64;
        let sr = sample_rate as f64;
        let t = current_time();
        let n = 5.0f64;
        let mut sum = 0.0;
        let mut weight_sum = 0.0;
        let half_w = (w.abs() * 0.5).max(0.001);
        let step = (2.0 * half_w) / (n - 1.0).max(1.0);
        for i in 0..n as i32 {
            let sample_time = t - half_w + step * i as f64;
            let weight = 1.0 - (sample_time - t).abs() / half_w;
            let val = (sample_time + half_w * 0.25) * sr;
            sum += val * weight.max(0.0);
            weight_sum += weight.max(0.0);
        }
        if weight_sum > 0.0 {
            sum / weight_sum
        } else {
            (t + half_w * 0.25) * sr
        }
    });
    engine.register_fn(
        "smooth",
        |width: f64, sample_rate: f64, samples: f64| -> f64 {
            let t = current_time();
            let n = samples.clamp(2.0, 64.0);
            let mut sum = 0.0;
            let mut weight_sum = 0.0;
            let half_w = (width.abs() * 0.5).max(0.001);
            let step = (2.0 * half_w) / (n - 1.0).max(1.0);
            for i in 0..n as i32 {
                let sample_time = t - half_w + step * i as f64;
                let weight = 1.0 - (sample_time - t).abs() / half_w;
                let val = (sample_time + half_w * 0.25) * sample_rate;
                sum += val * weight.max(0.0);
                weight_sum += weight.max(0.0);
            }
            if weight_sum > 0.0 {
                sum / weight_sum
            } else {
                (t + half_w * 0.25) * sample_rate
            }
        },
    );
    engine.register_fn(
        "smooth",
        |width: f64, sample_rate: f64, samples: i64| -> f64 {
            let t = current_time();
            let n = (samples as f64).clamp(2.0, 64.0);
            let mut sum = 0.0;
            let mut weight_sum = 0.0;
            let half_w = (width.abs() * 0.5).max(0.001);
            let step = (2.0 * half_w) / (n - 1.0).max(1.0);
            for i in 0..n as i32 {
                let sample_time = t - half_w + step * i as f64;
                let weight = 1.0 - (sample_time - t).abs() / half_w;
                let val = (sample_time + half_w * 0.25) * sample_rate;
                sum += val * weight.max(0.0);
                weight_sum += weight.max(0.0);
            }
            if weight_sum > 0.0 {
                sum / weight_sum
            } else {
                (t + half_w * 0.25) * sample_rate
            }
        },
    );
}
