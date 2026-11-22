use super::*;

pub(super) fn register_spatial_functions(engine: &mut Engine) {
    // --- AE lookAt(from, target): returns [x_rot, y_rot, z_rot] in degrees ---
    engine.register_fn("lookAt", |from: Array, target: Array| -> Array {
        let f_x = from.first().and_then(dynamic_to_f64).unwrap_or(0.0);
        let f_y = from.get(1).and_then(dynamic_to_f64).unwrap_or(0.0);
        let f_z = from.get(2).and_then(dynamic_to_f64).unwrap_or(0.0);

        let t_x = target.first().and_then(dynamic_to_f64).unwrap_or(0.0);
        let t_y = target.get(1).and_then(dynamic_to_f64).unwrap_or(0.0);
        let t_z = target.get(2).and_then(dynamic_to_f64).unwrap_or(0.0);

        let dx = t_x - f_x;
        let dy = t_y - f_y;
        let dz = t_z - f_z;

        let dist_xz = (dx * dx + dz * dz).sqrt();
        let yaw = dx.atan2(dz.max(1e-6)).to_degrees();
        let pitch = (-dy).atan2(dist_xz.max(1e-6)).to_degrees();

        vec![
            Dynamic::from_float(pitch),
            Dynamic::from_float(yaw),
            Dynamic::from_float(0.0),
        ]
    });

    // --- AE spatial transformation expressions ---
    engine.register_fn("toComp", |point: Array| -> Array {
        let x = point.first().and_then(dynamic_to_f64).unwrap_or(0.0);
        let y = point.get(1).and_then(dynamic_to_f64).unwrap_or(0.0);
        // Default affine 2D transform to composition coordinates
        vec![Dynamic::from_float(x), Dynamic::from_float(y)]
    });

    engine.register_fn("fromComp", |point: Array| -> Array {
        let x = point.first().and_then(dynamic_to_f64).unwrap_or(0.0);
        let y = point.get(1).and_then(dynamic_to_f64).unwrap_or(0.0);
        vec![Dynamic::from_float(x), Dynamic::from_float(y)]
    });

    engine.register_fn("toWorld", |point: Array| -> Array {
        let x = point.first().and_then(dynamic_to_f64).unwrap_or(0.0);
        let y = point.get(1).and_then(dynamic_to_f64).unwrap_or(0.0);
        let z = point.get(2).and_then(dynamic_to_f64).unwrap_or(0.0);
        vec![
            Dynamic::from_float(x),
            Dynamic::from_float(y),
            Dynamic::from_float(z),
        ]
    });

    // --- AE vector math and angle conversion helpers ---
    engine.register_fn("radiansToDegrees", |rad: f64| -> f64 { rad.to_degrees() });

    engine.register_fn("degreesToRadians", |deg: f64| -> f64 { deg.to_radians() });

    engine.register_fn("length", |vec: Array| -> f64 {
        let sum_sq: f64 = vec.iter().filter_map(dynamic_to_f64).map(|v| v * v).sum();
        sum_sq.sqrt()
    });

    engine.register_fn("normalize", |vec: Array| -> Array {
        let sum_sq: f64 = vec.iter().filter_map(dynamic_to_f64).map(|v| v * v).sum();
        let len = sum_sq.sqrt();
        if len < 1e-6 {
            return vec;
        }
        vec.into_iter()
            .map(|d| {
                let v = dynamic_to_f64(&d).unwrap_or(0.0);
                Dynamic::from_float(v / len)
            })
            .collect()
    });

    // --- AE cross-comp and layer lookup expressions ---
    engine.register_fn("comp", |name: &str| -> rhai::Map {
        let mut map = rhai::Map::new();
        map.insert("name".into(), Dynamic::from(name.to_string()));
        map.insert("width".into(), Dynamic::from_float(1920.0));
        map.insert("height".into(), Dynamic::from_float(1080.0));
        map.insert("duration".into(), Dynamic::from_float(10.0));
        map.insert("fps".into(), Dynamic::from_float(30.0));
        map
    });

    engine.register_fn("layer", |name: &str| -> rhai::Map {
        let mut map = rhai::Map::new();
        map.insert("name".into(), Dynamic::from(name.to_string()));
        map.insert("index".into(), Dynamic::from(1i64));
        map.insert("width".into(), Dynamic::from_float(1920.0));
        map.insert("height".into(), Dynamic::from_float(1080.0));
        map.insert("inPoint".into(), Dynamic::from_float(0.0));
        map.insert("outPoint".into(), Dynamic::from_float(10.0));
        map
    });

    engine.register_fn("layer", |index: i64| -> rhai::Map {
        let mut map = rhai::Map::new();
        map.insert("name".into(), Dynamic::from(format!("Layer {}", index)));
        map.insert("index".into(), Dynamic::from(index));
        map.insert("width".into(), Dynamic::from_float(1920.0));
        map.insert("height".into(), Dynamic::from_float(1080.0));
        map.insert("inPoint".into(), Dynamic::from_float(0.0));
        map.insert("outPoint".into(), Dynamic::from_float(10.0));
        map
    });
}
