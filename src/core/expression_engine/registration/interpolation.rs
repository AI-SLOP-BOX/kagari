use super::*;

pub(super) fn register(engine: &mut Engine) {
    // --- Math helpers mirroring AE's Math object ---
    engine.register_fn("sin", |x: f64| -> f64 { x.sin() });
    engine.register_fn("cos", |x: f64| -> f64 { x.cos() });
    engine.register_fn("abs", |x: f64| -> f64 { x.abs() });
    engine.register_fn("clamp", |v: f64, lo: f64, hi: f64| -> f64 {
        v.clamp(lo, hi)
    });
    // --- AE-compatible Linear interpolation (3-arg & 5-arg, scalar & array) ---
    engine.register_fn("linear", |t: f64, v_min: f64, v_max: f64| -> f64 {
        let s = t.clamp(0.0, 1.0);
        v_min + s * (v_max - v_min)
    });
    engine.register_fn(
        "linear",
        |t: f64, t_min: f64, t_max: f64, v_min: f64, v_max: f64| -> f64 {
            let s = if (t_max - t_min).abs() < 1e-6 {
                0.0
            } else {
                ((t - t_min) / (t_max - t_min)).clamp(0.0, 1.0)
            };
            v_min + s * (v_max - v_min)
        },
    );
    engine.register_fn("linear", |t: f64, a1: Array, a2: Array| -> Array {
        let s = t.clamp(0.0, 1.0);
        let n = a1.len().min(a2.len());
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let v1 = dynamic_to_f64(&a1[i]).unwrap_or(0.0);
            let v2 = dynamic_to_f64(&a2[i]).unwrap_or(0.0);
            out.push(Dynamic::from_float(v1 + s * (v2 - v1)));
        }
        out
    });
    engine.register_fn(
        "linear",
        |t: f64, t_min: f64, t_max: f64, a1: Array, a2: Array| -> Array {
            let s = if (t_max - t_min).abs() < 1e-6 {
                0.0
            } else {
                ((t - t_min) / (t_max - t_min)).clamp(0.0, 1.0)
            };
            let n = a1.len().min(a2.len());
            let mut out = Vec::with_capacity(n);
            for i in 0..n {
                let v1 = dynamic_to_f64(&a1[i]).unwrap_or(0.0);
                let v2 = dynamic_to_f64(&a2[i]).unwrap_or(0.0);
                out.push(Dynamic::from_float(v1 + s * (v2 - v1)));
            }
            out
        },
    );

    // --- AE-compatible Easing functions (3-arg & 5-arg, scalar & array) ---
    engine.register_fn("ease", |t: f64, v_min: f64, v_max: f64| -> f64 {
        let s = t.clamp(0.0, 1.0);
        let smooth_s = s * s * (3.0 - 2.0 * s);
        v_min + smooth_s * (v_max - v_min)
    });
    engine.register_fn(
        "ease",
        |t: f64, t_min: f64, t_max: f64, v_min: f64, v_max: f64| -> f64 {
            let s = if (t_max - t_min).abs() < 1e-6 {
                0.0
            } else {
                ((t - t_min) / (t_max - t_min)).clamp(0.0, 1.0)
            };
            let smooth_s = s * s * (3.0 - 2.0 * s); // Hermite smoothstep
            v_min + smooth_s * (v_max - v_min)
        },
    );
    engine.register_fn(
        "ease",
        |t: f64, t_min: f64, t_max: f64, a1: Array, a2: Array| -> Array {
            let s = if (t_max - t_min).abs() < 1e-6 {
                0.0
            } else {
                ((t - t_min) / (t_max - t_min)).clamp(0.0, 1.0)
            };
            let smooth_s = s * s * (3.0 - 2.0 * s);
            let n = a1.len().min(a2.len());
            let mut out = Vec::with_capacity(n);
            for i in 0..n {
                let v1 = dynamic_to_f64(&a1[i]).unwrap_or(0.0);
                let v2 = dynamic_to_f64(&a2[i]).unwrap_or(0.0);
                out.push(Dynamic::from_float(v1 + smooth_s * (v2 - v1)));
            }
            out
        },
    );
    engine.register_fn("ease", |t: f64, a1: Array, a2: Array| -> Array {
        let s = t.clamp(0.0, 1.0);
        let smooth_s = s * s * (3.0 - 2.0 * s);
        let n = a1.len().min(a2.len());
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let v1 = dynamic_to_f64(&a1[i]).unwrap_or(0.0);
            let v2 = dynamic_to_f64(&a2[i]).unwrap_or(0.0);
            out.push(Dynamic::from_float(v1 + smooth_s * (v2 - v1)));
        }
        out
    });

    engine.register_fn("easeIn", |t: f64, v_min: f64, v_max: f64| -> f64 {
        let s = t.clamp(0.0, 1.0);
        v_min + s * s * (v_max - v_min)
    });
    engine.register_fn(
        "easeIn",
        |t: f64, t_min: f64, t_max: f64, v_min: f64, v_max: f64| -> f64 {
            let s = if (t_max - t_min).abs() < 1e-6 {
                0.0
            } else {
                ((t - t_min) / (t_max - t_min)).clamp(0.0, 1.0)
            };
            let ease_in_s = s * s; // Quadratic acceleration
            v_min + ease_in_s * (v_max - v_min)
        },
    );
    engine.register_fn(
        "easeIn",
        |t: f64, t_min: f64, t_max: f64, a1: Array, a2: Array| -> Array {
            let s = if (t_max - t_min).abs() < 1e-6 {
                0.0
            } else {
                ((t - t_min) / (t_max - t_min)).clamp(0.0, 1.0)
            };
            let ease_in_s = s * s;
            let n = a1.len().min(a2.len());
            let mut out = Vec::with_capacity(n);
            for i in 0..n {
                let v1 = dynamic_to_f64(&a1[i]).unwrap_or(0.0);
                let v2 = dynamic_to_f64(&a2[i]).unwrap_or(0.0);
                out.push(Dynamic::from_float(v1 + ease_in_s * (v2 - v1)));
            }
            out
        },
    );

    engine.register_fn("easeOut", |t: f64, v_min: f64, v_max: f64| -> f64 {
        let s = t.clamp(0.0, 1.0);
        let ease_out_s = 1.0 - (1.0 - s) * (1.0 - s);
        v_min + ease_out_s * (v_max - v_min)
    });
    engine.register_fn(
        "easeOut",
        |t: f64, t_min: f64, t_max: f64, v_min: f64, v_max: f64| -> f64 {
            let s = if (t_max - t_min).abs() < 1e-6 {
                0.0
            } else {
                ((t - t_min) / (t_max - t_min)).clamp(0.0, 1.0)
            };
            let ease_out_s = 1.0 - (1.0 - s) * (1.0 - s); // Quadratic deceleration
            v_min + ease_out_s * (v_max - v_min)
        },
    );
    engine.register_fn(
        "easeOut",
        |t: f64, t_min: f64, t_max: f64, a1: Array, a2: Array| -> Array {
            let s = if (t_max - t_min).abs() < 1e-6 {
                0.0
            } else {
                ((t - t_min) / (t_max - t_min)).clamp(0.0, 1.0)
            };
            let ease_out_s = 1.0 - (1.0 - s) * (1.0 - s);
            let n = a1.len().min(a2.len());
            let mut out = Vec::with_capacity(n);
            for i in 0..n {
                let v1 = dynamic_to_f64(&a1[i]).unwrap_or(0.0);
                let v2 = dynamic_to_f64(&a2[i]).unwrap_or(0.0);
                out.push(Dynamic::from_float(v1 + ease_out_s * (v2 - v1)));
            }
            out
        },
    );
}
