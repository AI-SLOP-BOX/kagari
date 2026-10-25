use super::*;

type GraphSamples = (Vec<(u32, f32)>, Vec<(u32, f32)>);

pub(super) fn sample_graph_property(layer: &Layer, graph_prop: &str, total_f: u32) -> GraphSamples {
    // Sample values along timeline duration for drawing curve (screen-adaptive step)
    let max_samples = 2000usize;
    let step = (total_f as usize / max_samples).max(1) as u32;
    let mut samples = Vec::with_capacity((total_f / step) as usize + 2);
    let mut f = 0u32;
    while f <= total_f {
        let raw_val = match graph_prop {
            "Position X" => layer.transform.position.evaluate(f)[0],
            "Position Y" => layer.transform.position.evaluate(f)[1],
            "Scale X" => layer.transform.scale.evaluate(f)[0],
            "Scale Y" => layer.transform.scale.evaluate(f)[1],
            "Rotation" => layer.transform.rotation.evaluate(f),
            "Opacity" => layer.transform.opacity.evaluate(f),
            p if p.starts_with("3D Position") => {
                layer.transform_3d.position.evaluate(f)[axis_3d(p)]
            }
            p if p.starts_with("3D Rotation") => {
                layer.transform_3d.rotation.evaluate(f)[axis_3d(p)]
            }
            p if p.starts_with("3D Scale") => layer.transform_3d.scale.evaluate(f)[axis_3d(p)],
            p if p.starts_with("PinX:") || p.starts_with("PinY:") => {
                let ci = pin_component(p);
                let pid = p.split(':').nth(1).unwrap_or("");
                layer
                    .puppet_pins
                    .iter()
                    .find(|pp| pp.id == pid)
                    .map(|pp| pp.position.evaluate(f)[ci])
                    .unwrap_or(0.0)
            }
            p if is_effect_property(p) => effect_parameter_channel_value(layer, p, f),
            _ => layer.transform.position.evaluate(f)[0],
        };
        let val = if raw_val.is_nan() { 0.0 } else { raw_val };
        samples.push((f, val));
        if f < total_f && f + step > total_f {
            f = total_f;
        } else {
            f += step;
        }
    }

    // Compute per-keyframe velocity when in speed graph mode
    let keyframes_ref: Vec<(u32, f32)> = match graph_prop {
        "Position X" => layer
            .transform
            .position
            .keyframes()
            .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value[0])).collect())
            .unwrap_or_default(),
        "Position Y" => layer
            .transform
            .position
            .keyframes()
            .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value[1])).collect())
            .unwrap_or_default(),
        "Scale X" => layer
            .transform
            .scale
            .keyframes()
            .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value[0])).collect())
            .unwrap_or_default(),
        "Scale Y" => layer
            .transform
            .scale
            .keyframes()
            .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value[1])).collect())
            .unwrap_or_default(),
        "Rotation" => layer
            .transform
            .rotation
            .keyframes()
            .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value)).collect())
            .unwrap_or_default(),
        "Opacity" => layer
            .transform
            .opacity
            .keyframes()
            .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value)).collect())
            .unwrap_or_default(),
        p if p.starts_with("3D Position") => layer
            .transform_3d
            .position
            .keyframes()
            .map(|kfs| {
                kfs.iter()
                    .map(|kf| (kf.frame, kf.value[axis_3d(p)]))
                    .collect()
            })
            .unwrap_or_default(),
        p if p.starts_with("3D Rotation") => layer
            .transform_3d
            .rotation
            .keyframes()
            .map(|kfs| {
                kfs.iter()
                    .map(|kf| (kf.frame, kf.value[axis_3d(p)]))
                    .collect()
            })
            .unwrap_or_default(),
        p if p.starts_with("3D Scale") => layer
            .transform_3d
            .scale
            .keyframes()
            .map(|kfs| {
                kfs.iter()
                    .map(|kf| (kf.frame, kf.value[axis_3d(p)]))
                    .collect()
            })
            .unwrap_or_default(),
        p if p.starts_with("PinX:") || p.starts_with("PinY:") => {
            let ci = pin_component(p);
            let pid = p.split(':').nth(1).unwrap_or("");
            layer
                .puppet_pins
                .iter()
                .find(|pp| pp.id == pid)
                .and_then(|pp| pp.position.keyframes())
                .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value[ci])).collect())
                .unwrap_or_default()
        }
        p if is_effect_property(p) => effect_parameter_channel_keyframes(layer, p),
        _ => vec![],
    };

    (samples, keyframes_ref)
}
