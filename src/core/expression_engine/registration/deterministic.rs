use super::*;

pub(super) fn register(engine: &mut Engine) {
    // --- AE-compatible Random functions (PCG32 Deterministic Generator with time-variation) ---
    engine.register_fn("random", |min: f64, max: f64| -> f64 {
        let seed = min.to_bits() ^ max.to_bits().rotate_left(16);
        let r = pcg32_hash(seed);
        min + r * (max - min)
    });

    engine.register_fn("random", |time: f64, min: f64, max: f64| -> f64 {
        let seed = time.to_bits() ^ min.to_bits() ^ max.to_bits().rotate_left(16);
        let r = pcg32_hash(seed);
        min + r * (max - min)
    });

    engine.register_fn("gaussRandom", |min: f64, max: f64| -> f64 {
        let mean = (min + max) * 0.5;
        let std_dev = (max - min) * 0.16666;
        let seed1 = min.to_bits() ^ 0xa09e667f3bcc9091;
        let seed2 = max.to_bits() ^ 0x517cc1b727220a95;
        let u1 = pcg32_hash(seed1).clamp(0.0001, 0.9999);
        let u2 = pcg32_hash(seed2).clamp(0.0001, 0.9999);
        let z0 = f64::sqrt(-2.0 * u1.ln()) * (std::f64::consts::TAU * u2).cos();
        (mean + z0 * std_dev).clamp(min, max)
    });

    engine.register_fn("gaussRandom", |time: f64, min: f64, max: f64| -> f64 {
        let mean = (min + max) * 0.5;
        let std_dev = (max - min) * 0.16666;
        let t_bits = time.to_bits();
        let seed1 = min.to_bits() ^ t_bits ^ 0xa09e667f3bcc9091;
        let seed2 = max.to_bits() ^ t_bits.rotate_left(32) ^ 0x517cc1b727220a95;
        let u1 = pcg32_hash(seed1).clamp(0.0001, 0.9999);
        let u2 = pcg32_hash(seed2).clamp(0.0001, 0.9999);
        let z0 = f64::sqrt(-2.0 * u1.ln()) * (std::f64::consts::TAU * u2).cos();
        (mean + z0 * std_dev).clamp(min, max)
    });

    // --- Array constructors ---
    engine.register_fn("array2", |x: f64, y: f64| -> Array {
        vec![Dynamic::from_float(x), Dynamic::from_float(y)]
    });
    engine.register_fn("array3", |x: f64, y: f64, z: f64| -> Array {
        vec![
            Dynamic::from_float(x),
            Dynamic::from_float(y),
            Dynamic::from_float(z),
        ]
    });
    engine.register_fn("array4", |x: f64, y: f64, z: f64, w: f64| -> Array {
        vec![
            Dynamic::from_float(x),
            Dynamic::from_float(y),
            Dynamic::from_float(z),
            Dynamic::from_float(w),
        ]
    });

    // --- Additional AE-compatible math functions ---
    engine.register_fn("ceil", |x: f64| -> f64 { x.ceil() });
    engine.register_fn("floor", |x: f64| -> f64 { x.floor() });
    engine.register_fn("round", |x: f64| -> f64 { x.round() });
    engine.register_fn("min", |a: f64, b: f64| -> f64 { a.min(b) });
    engine.register_fn("max", |a: f64, b: f64| -> f64 { a.max(b) });
    engine.register_fn("pow", |base: f64, exp: f64| -> f64 { base.powf(exp) });
    engine.register_fn("sqrt", |x: f64| -> f64 { x.sqrt() });
    engine.register_fn("sinh", |x: f64| -> f64 { x.sinh() });
    engine.register_fn("cosh", |x: f64| -> f64 { x.cosh() });
    engine.register_fn("tanh", |x: f64| -> f64 { x.tanh() });
    engine.register_fn("asin", |x: f64| -> f64 { x.asin() });
    engine.register_fn("acos", |x: f64| -> f64 { x.acos() });
    engine.register_fn("atan", |x: f64| -> f64 { x.atan() });
    engine.register_fn("atan2", |y: f64, x: f64| -> f64 { y.atan2(x) });
    engine.register_fn("log", |x: f64| -> f64 { x.ln() });
    engine.register_fn("log10", |x: f64| -> f64 { x.log10() });
    engine.register_fn("exp", |x: f64| -> f64 { x.exp() });

    // --- AE trigonometry in degrees ---
    engine.register_fn("degreesToRadians", |d: f64| -> f64 { d.to_radians() });
    engine.register_fn("radiansToDegrees", |r: f64| -> f64 { r.to_degrees() });

    // --- Interpolation (more flexible than linear) ---
    engine.register_fn(
        "interpolate",
        |t: f64, t1: f64, t2: f64, v1: f64, v2: f64| -> f64 {
            let s = if (t2 - t1).abs() < 1e-6 {
                0.0
            } else {
                ((t - t1) / (t2 - t1)).clamp(0.0, 1.0)
            };
            v1 + s * (v2 - v1)
        },
    );

    // --- Smoothstep (AE-compatible) ---
    engine.register_fn("smoothstep", |edge0: f64, edge1: f64, x: f64| -> f64 {
        let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    });

    // --- Distance between two points (1D) ---
    engine.register_fn("distance", |a: f64, b: f64| -> f64 { (a - b).abs() });

    // --- Ping-pong function ---
    engine.register_fn("pingpong", |t: f64, max: f64| -> f64 {
        let t_mod = t.rem_euclid(max * 2.0);
        if t_mod > max {
            max * 2.0 - t_mod
        } else {
            t_mod
        }
    });

    // --- Wrap function (AE-style) ---
    engine.register_fn("wrap", |val: f64, min_val: f64, max_val: f64| -> f64 {
        let range = max_val - min_val;
        if range.abs() < 1e-6 {
            return min_val;
        }
        let v = (val - min_val).rem_euclid(range);
        min_val + v
    });

    // --- AE seedRandom (deterministic random based on seed) ---
    engine.register_fn("seedRandom", |seed: f64| -> f64 {
        let bits = if seed.is_finite() {
            (seed.abs() * 1000.0) as u64
        } else {
            0
        };
        pcg32_hash(bits)
    });
    engine.register_fn("seedRandom", |seed: f64, timeless: bool| -> f64 {
        let _ = timeless;
        let bits = if seed.is_finite() {
            (seed.abs() * 1000.0) as u64
        } else {
            0
        };
        pcg32_hash(bits)
    });

    // --- Noise functions (simple value noise) ---
    engine.register_fn("noise", |x: f64| -> f64 {
        let n = x.sin() * 43758.5453;
        n - n.floor()
    });
    engine.register_fn("noise", |x: f64, y: f64| -> f64 {
        let n = (x * 12.9898 + y * 78.233).sin() * 43758.5453;
        n - n.floor()
    });

    // --- Toe / Shoulder (filmic helpers) ---
    engine.register_fn("toe", |x: f64, strength: f64| -> f64 {
        let p = strength.max(0.001);
        x.powf(p)
    });
    engine.register_fn("shoulder", |x: f64, strength: f64| -> f64 {
        let p = strength.max(0.001);
        1.0 - (1.0 - x).powf(p)
    });
}
