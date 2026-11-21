use super::*;

pub(super) fn hex_to_rgba(s: &str) -> [f32; 4] {
    let h = s.trim_start_matches('#');
    let n = u32::from_str_radix(h, 16).unwrap_or(0);
    match h.len() {
        8 => [
            ((n >> 24) & 255) as f32 / 255.0,
            ((n >> 16) & 255) as f32 / 255.0,
            ((n >> 8) & 255) as f32 / 255.0,
            (n & 255) as f32 / 255.0,
        ],
        _ => [
            ((n >> 16) & 255) as f32 / 255.0,
            ((n >> 8) & 255) as f32 / 255.0,
            (n & 255) as f32 / 255.0,
            1.0,
        ],
    }
}

/// Reads a static 2-component value: `{"a":0,"k":[x,y]}` or bare `[x,y]`.
pub(super) fn k_arr2(v: Option<&Value>) -> [f32; 2] {
    let Some(val) = v else {
        return [0.0; 2];
    };
    let k = val.get("k").unwrap_or(val);
    let Some(arr) = k.as_array() else {
        return [0.0; 2];
    };
    if arr.len() < 2 {
        return [0.0; 2];
    }
    [
        arr[0].as_f64().unwrap_or(0.0) as f32,
        arr[1].as_f64().unwrap_or(0.0) as f32,
    ]
}

/// Reads a static scalar value: `{"a":0,"k":v}`, `"k":v` or bare `v`.
pub(super) fn k_f32(v: Option<&Value>, default: f32) -> f32 {
    let Some(val) = v else {
        return default;
    };
    let k = val.get("k").unwrap_or(val);
    k.as_f64().map(|f| f as f32).unwrap_or(default)
}

/// Reads fill/stroke color `[r,g,b]` or `[r,g,b,a]` (0..1 floats).
pub(super) fn rgb_from_k(v: Option<&Value>) -> [f32; 4] {
    let Some(val) = v else {
        return [0.0; 4];
    };
    let k = val.get("k").unwrap_or(val);
    let Some(arr) = k.as_array() else {
        return [0.0; 4];
    };
    let comp = |i: usize| arr.get(i).and_then(Value::as_f64).unwrap_or(0.0) as f32;
    [
        comp(0),
        comp(1),
        comp(2),
        if arr.len() > 3 { comp(3) } else { 1.0 },
    ]
}

/// Inverse of the exporter's anim_property_* writers: `{"a","k"}` → Animatable.
/// Terminal bodymovin keyframes lacking "s" hold the previous value.
pub(super) fn parse_anim_value_f32(v: Option<&Value>) -> Animatable<f32> {
    let Some(val) = v else {
        return Animatable::new_constant(0.0);
    };
    if val.get("a").and_then(Value::as_i64) == Some(1) {
        if let Some(list) = val.get("k").and_then(Value::as_array) {
            let mut kfs: Vec<Keyframe<f32>> = Vec::new();
            let mut prev = 0.0f32;
            for e in list {
                let frame = e.get("t").and_then(Value::as_f64).unwrap_or(0.0).max(0.0) as u32;
                let value = e
                    .get("s")
                    .and_then(Value::as_array)
                    .and_then(|s| s.first())
                    .and_then(Value::as_f64)
                    .map(|f| f as f32)
                    .unwrap_or(prev);
                prev = value;
                kfs.push(Keyframe::new(frame, value, InterpolationType::Linear));
            }
            if !kfs.is_empty() {
                return Animatable::Animated(kfs);
            }
        }
    }
    Animatable::new_constant(val.get("k").and_then(Value::as_f64).unwrap_or(0.0) as f32)
}

pub(super) fn parse_anim_value_v2(v: Option<&Value>) -> Animatable<[f32; 2]> {
    let Some(val) = v else {
        return Animatable::new_constant([0.0, 0.0]);
    };
    if val.get("a").and_then(Value::as_i64) == Some(1) {
        if let Some(list) = val.get("k").and_then(Value::as_array) {
            let mut kfs: Vec<Keyframe<[f32; 2]>> = Vec::new();
            let mut prev = [0.0f32; 2];
            for e in list {
                let frame = e.get("t").and_then(Value::as_f64).unwrap_or(0.0).max(0.0) as u32;
                let value = e
                    .get("s")
                    .and_then(Value::as_array)
                    .map(|s| {
                        // Handle both s: [[x, y]] and s: [x, y] formats
                        if let Some(first) = s.first() {
                            if let Some(arr) = first.as_array() {
                                // Nested: s: [[x, y]]
                                [
                                    arr.first()
                                        .and_then(Value::as_f64)
                                        .unwrap_or(prev[0] as f64)
                                        as f32,
                                    arr.get(1).and_then(Value::as_f64).unwrap_or(prev[1] as f64)
                                        as f32,
                                ]
                            } else {
                                // Flat: s: [x, y]
                                [
                                    s.first().and_then(Value::as_f64).unwrap_or(prev[0] as f64)
                                        as f32,
                                    s.get(1).and_then(Value::as_f64).unwrap_or(prev[1] as f64)
                                        as f32,
                                ]
                            }
                        } else {
                            prev
                        }
                    })
                    .unwrap_or(prev);
                prev = value;
                kfs.push(Keyframe::new(frame, value, InterpolationType::Linear));
            }
            if !kfs.is_empty() {
                return Animatable::Animated(kfs);
            }
        }
    }
    Animatable::new_constant(k_arr2(Some(val)))
}

pub(super) fn parse_transform(ks: &Value) -> crate::core::timeline::Transform2D {
    crate::core::timeline::Transform2D {
        anchor_point: parse_anim_value_v2(ks.get("a")),
        position: parse_anim_value_v2(ks.get("p")),
        scale: parse_anim_value_v2(ks.get("s")),
        rotation: parse_anim_value_f32(ks.get("r")),
        opacity: parse_anim_value_f32(ks.get("o")),
        ..Default::default()
    }
}

pub(super) fn blend_from_code(code: i64) -> BlendMode {
    match code {
        1 => BlendMode::Multiply,
        2 => BlendMode::Screen,
        3 => BlendMode::Overlay,
        4 => BlendMode::Add,
        _ => BlendMode::Normal,
    }
}

pub(super) fn matte_from_code(code: i64) -> TrackMatteMode {
    match code {
        1 => TrackMatteMode::AlphaMatte,
        2 => TrackMatteMode::AlphaMatteInverted,
        3 => TrackMatteMode::LumaMatte,
        4 => TrackMatteMode::LumaMatteInverted,
        _ => TrackMatteMode::None,
    }
}

/// Reconstructs a Text layer from one bodymovin text-document keyframe.
pub(super) fn text_layer_type(doc: &Value) -> LayerType {
    let s = doc.get("s").cloned().unwrap_or(Value::Null);
    let size = s.get("s").and_then(Value::as_f64).unwrap_or(24.0).max(1.0) as u32;
    let lh = s
        .get("lh")
        .and_then(Value::as_f64)
        .unwrap_or(size as f64 * 1.2);
    // Lottie j 0=Left 2=Center 1=Right → app align 0=Left 1=Center 2=Right.
    let align = match s.get("j").and_then(Value::as_i64).unwrap_or(0) {
        2 => 1u32,
        1 => 2,
        _ => 0,
    };
    LayerType::Text {
        text: s.get("t").and_then(Value::as_str).unwrap_or("").to_string(),
        font_size: size,
        color: rgb_from_k(s.get("fc")),
        font_family: s
            .get("f")
            .and_then(Value::as_str)
            .unwrap_or("Inter")
            .to_string(),
        tracking: s.get("tr").and_then(Value::as_f64).unwrap_or(0.0) as f32,
        leading: (lh / (size as f64).max(1.0)) as f32,
        align: align as usize,
        stroke_color: [0.0; 4],
        stroke_width: 0.0,
        text_on_path: false,
    }
}

/// Parses the first shape group into our ShapeType plus fill/stroke.
/// Animated shape sizes degrade to their first keyframe value.
pub(super) fn parse_shape_group(shapes: &Value) -> Option<(ShapeType, [f32; 4], [f32; 4], f32)> {
    for g in shapes.as_array()? {
        let Some(it) = g.get("it").and_then(Value::as_array) else {
            continue;
        };
        let mut geo: Option<ShapeType> = None;
        let mut fill = [0.0f32; 4];
        let mut stroke = [0.0f32; 4];
        let mut stroke_w = 0.0f32;
        for item in it {
            match item.get("ty").and_then(Value::as_str).unwrap_or("") {
                "el" => {
                    let s = k_arr2(item.get("s"));
                    geo = Some(ShapeType::Ellipse {
                        width: Animatable::new_constant(s[0]),
                        height: Animatable::new_constant(s[1]),
                    });
                }
                "rc" => {
                    let s = k_arr2(item.get("s"));
                    geo = Some(ShapeType::Rectangle {
                        width: Animatable::new_constant(s[0]),
                        height: Animatable::new_constant(s[1]),
                        corner_radius: Animatable::new_constant(k_f32(item.get("r"), 0.0)),
                    });
                }
                "sr" => {
                    let sy = item.get("sy").and_then(Value::as_i64).unwrap_or(1);
                    let points = k_f32(item.get("pt"), 5.0);
                    let outer = k_f32(item.get("or"), 50.0);
                    let inner = k_f32(item.get("ir"), outer * 0.5);
                    geo = Some(if sy == 2 {
                        ShapeType::Star {
                            points: Animatable::new_constant(points),
                            inner_radius: Animatable::new_constant(inner),
                            outer_radius: Animatable::new_constant(outer),
                        }
                    } else {
                        ShapeType::Polygon {
                            sides: Animatable::new_constant(points),
                            radius: Animatable::new_constant(outer),
                        }
                    });
                }
                "fl" => fill = rgb_from_k(item.get("c")),
                "st" => {
                    stroke = rgb_from_k(item.get("c"));
                    stroke_w = k_f32(item.get("w"), 0.0);
                }
                _ => {}
            }
        }
        if let Some(shape_type) = geo {
            return Some((shape_type, fill, stroke, stroke_w));
        }
    }
    None
}

pub(super) fn pt2(v: Option<&Value>) -> [f32; 2] {
    let Some(a) = v.and_then(Value::as_array) else {
        return [0.0; 2];
    };
    [
        a.first().and_then(Value::as_f64).unwrap_or(0.0) as f32,
        a.get(1).and_then(Value::as_f64).unwrap_or(0.0) as f32,
    ]
}

pub(super) fn shape_vertices(sh: &Value) -> Vec<[f32; 2]> {
    sh.get("v")
        .and_then(Value::as_array)
        .map(|a| a.iter().map(|p| pt2(Some(p))).collect())
        .unwrap_or_default()
}

/// Per-vertex tangent handles; None when every handle is zero so that
/// export→import→export round-trips stay lossless.
pub(super) fn shape_tangents(sh: &Value, vert_count: usize) -> Option<Vec<([f32; 2], [f32; 2])>> {
    let ins: Vec<[f32; 2]> = sh
        .get("i")
        .and_then(Value::as_array)
        .map(|a| a.iter().map(|p| pt2(Some(p))).collect())
        .unwrap_or_else(|| vec![[0.0; 2]; vert_count]);
    let outs: Vec<[f32; 2]> = sh
        .get("o")
        .and_then(Value::as_array)
        .map(|a| a.iter().map(|p| pt2(Some(p))).collect())
        .unwrap_or_else(|| vec![[0.0; 2]; vert_count]);
    let mut any = false;
    let pairs: Vec<([f32; 2], [f32; 2])> = (0..vert_count)
        .map(|i| {
            let tin = ins.get(i).copied().unwrap_or([0.0; 2]);
            let tout = outs.get(i).copied().unwrap_or([0.0; 2]);
            if tin != [0.0; 2] || tout != [0.0; 2] {
                any = true;
            }
            (tin, tout)
        })
        .collect();
    if any {
        Some(pairs)
    } else {
        None
    }
}

pub(super) fn import_mask(m: &Value, idx: usize) -> Mask {
    let letter = m.get("mode").and_then(Value::as_str).unwrap_or("a");
    let enabled = letter != "n";
    let mode = match letter {
        "s" => MaskMode::Subtract,
        "i" => MaskMode::Intersect,
        "l" => MaskMode::Lighten,
        "d" => MaskMode::Darken,
        "f" => MaskMode::Difference,
        _ => {
            if enabled {
                MaskMode::Add
            } else {
                MaskMode::None
            }
        }
    };

    let pt = m.get("pt");
    let is_closed = pt
        .and_then(|p| p.get("k"))
        .and_then(|k| k.get("c"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let animated = pt.and_then(|p| p.get("a")).and_then(Value::as_i64) == Some(1);

    let vertices: Animatable<Vec<[f32; 2]>> = if animated {
        let mut kfs: Vec<Keyframe<Vec<[f32; 2]>>> = Vec::new();
        if let Some(list) = pt.and_then(|p| p.get("k")).and_then(Value::as_array) {
            let mut prev: Vec<[f32; 2]> = Vec::new();
            for e in list {
                let frame = e.get("t").and_then(Value::as_f64).unwrap_or(0.0).max(0.0) as u32;
                let value = e
                    .get("s")
                    .and_then(Value::as_array)
                    .and_then(|s| s.first())
                    .map(shape_vertices)
                    .unwrap_or_else(|| prev.clone());
                prev = value.clone();
                kfs.push(Keyframe::new(frame, value, InterpolationType::Linear));
            }
        }
        if kfs.is_empty() {
            Animatable::new_constant(Vec::new())
        } else {
            Animatable::Animated(kfs)
        }
    } else {
        let verts = pt
            .and_then(|p| p.get("k"))
            .map(shape_vertices)
            .unwrap_or_default();
        Animatable::new_constant(verts)
    };

    // Tangents from whichever representation carries vertices. Animated
    // handles are rare; positions carry the essential shape there.
    let tangents = if animated {
        None
    } else {
        pt.and_then(|p| p.get("k")).and_then(|sh| {
            let n = shape_vertices(sh).len();
            shape_tangents(sh, n)
        })
    };

    Mask {
        id: format!("mask_{idx}"),
        name: m
            .get("nm")
            .and_then(Value::as_str)
            .unwrap_or("Mask")
            .to_string(),
        enabled,
        mode,
        path: MaskPath {
            vertices,
            tangents,
            is_closed,
        },
        feather: Animatable::new_constant(0.0),
        opacity: parse_anim_value_f32(m.get("o")),
        expansion: parse_anim_value_f32(m.get("x")),
        inverted: m.get("inv").and_then(Value::as_bool).unwrap_or(false),
        wiggle: None,
    }
}
