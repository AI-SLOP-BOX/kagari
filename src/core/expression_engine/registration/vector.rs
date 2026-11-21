use super::*;

pub(super) fn register(engine: &mut Engine) {
    // --- AE Vector Math (length, distance, normalize, dot, cross, lookAt, add, sub, mul, div) ---
    engine.register_fn("length", |a: Array| -> f64 {
        let sum_sq: f64 = a.iter().filter_map(dynamic_to_f64).map(|v| v * v).sum();
        sum_sq.sqrt()
    });
    engine.register_fn("length", |a1: Array, a2: Array| -> f64 {
        let n = a1.len().min(a2.len());
        let mut sum_sq = 0.0;
        for i in 0..n {
            let v1 = dynamic_to_f64(&a1[i]).unwrap_or(0.0);
            let v2 = dynamic_to_f64(&a2[i]).unwrap_or(0.0);
            let d = v1 - v2;
            sum_sq += d * d;
        }
        sum_sq.sqrt()
    });
    engine.register_fn("distance", |a1: Array, a2: Array| -> f64 {
        let n = a1.len().min(a2.len());
        let mut sum_sq = 0.0;
        for i in 0..n {
            let v1 = dynamic_to_f64(&a1[i]).unwrap_or(0.0);
            let v2 = dynamic_to_f64(&a2[i]).unwrap_or(0.0);
            let d = v1 - v2;
            sum_sq += d * d;
        }
        sum_sq.sqrt()
    });
    engine.register_fn("normalize", |a: Array| -> Array {
        let len: f64 = a
            .iter()
            .filter_map(dynamic_to_f64)
            .map(|v| v * v)
            .sum::<f64>()
            .sqrt();
        if len < 1e-9 {
            a
        } else {
            a.iter()
                .filter_map(dynamic_to_f64)
                .map(|v| Dynamic::from_float(v / len))
                .collect()
        }
    });
    engine.register_fn("dot", |a1: Array, a2: Array| -> f64 {
        let n = a1.len().min(a2.len());
        let mut sum = 0.0;
        for i in 0..n {
            let v1 = dynamic_to_f64(&a1[i]).unwrap_or(0.0);
            let v2 = dynamic_to_f64(&a2[i]).unwrap_or(0.0);
            sum += v1 * v2;
        }
        sum
    });
    engine.register_fn("cross", |a1: Array, a2: Array| -> Array {
        let v1_0 = a1.first().and_then(dynamic_to_f64).unwrap_or(0.0);
        let v1_1 = a1.get(1).and_then(dynamic_to_f64).unwrap_or(0.0);
        let v1_2 = a1.get(2).and_then(dynamic_to_f64).unwrap_or(0.0);
        let v2_0 = a2.first().and_then(dynamic_to_f64).unwrap_or(0.0);
        let v2_1 = a2.get(1).and_then(dynamic_to_f64).unwrap_or(0.0);
        let v2_2 = a2.get(2).and_then(dynamic_to_f64).unwrap_or(0.0);
        vec![
            Dynamic::from_float(v1_1 * v2_2 - v1_2 * v2_1),
            Dynamic::from_float(v1_2 * v2_0 - v1_0 * v2_2),
            Dynamic::from_float(v1_0 * v2_1 - v1_1 * v2_0),
        ]
    });
    engine.register_fn("lookAt", |from_p: Array, target_p: Array| -> Array {
        let fx = from_p.first().and_then(dynamic_to_f64).unwrap_or(0.0);
        let fy = from_p.get(1).and_then(dynamic_to_f64).unwrap_or(0.0);
        let fz = from_p.get(2).and_then(dynamic_to_f64).unwrap_or(0.0);
        let tx = target_p.first().and_then(dynamic_to_f64).unwrap_or(0.0);
        let ty = target_p.get(1).and_then(dynamic_to_f64).unwrap_or(0.0);
        let tz = target_p.get(2).and_then(dynamic_to_f64).unwrap_or(0.0);
        let dx = tx - fx;
        let dy = ty - fy;
        let dz = tz - fz;
        let x_rot = (-dy).atan2((dx * dx + dz * dz).sqrt()).to_degrees();
        let y_rot = dx.atan2(dz).to_degrees();
        vec![
            Dynamic::from_float(x_rot),
            Dynamic::from_float(y_rot),
            Dynamic::from_float(0.0),
        ]
    });
    engine.register_fn("add", |a1: Array, a2: Array| -> Array {
        let n = a1.len().min(a2.len());
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let v1 = dynamic_to_f64(&a1[i]).unwrap_or(0.0);
            let v2 = dynamic_to_f64(&a2[i]).unwrap_or(0.0);
            out.push(Dynamic::from_float(v1 + v2));
        }
        out
    });
    engine.register_fn("sub", |a1: Array, a2: Array| -> Array {
        let n = a1.len().min(a2.len());
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let v1 = dynamic_to_f64(&a1[i]).unwrap_or(0.0);
            let v2 = dynamic_to_f64(&a2[i]).unwrap_or(0.0);
            out.push(Dynamic::from_float(v1 - v2));
        }
        out
    });
    engine.register_fn("mul", |a: Array, s: f64| -> Array {
        a.iter()
            .filter_map(dynamic_to_f64)
            .map(|v| Dynamic::from_float(v * s))
            .collect()
    });
    engine.register_fn("div", |a: Array, s: f64| -> Array {
        let s_inv = if s.abs() < 1e-9 { 1.0 } else { 1.0 / s };
        a.iter()
            .filter_map(dynamic_to_f64)
            .map(|v| Dynamic::from_float(v * s_inv))
            .collect()
    });
    engine.register_fn("clamp", |a: Array, lo: f64, hi: f64| -> Array {
        a.iter()
            .filter_map(dynamic_to_f64)
            .map(|v| Dynamic::from_float(v.clamp(lo, hi)))
            .collect()
    });
}
