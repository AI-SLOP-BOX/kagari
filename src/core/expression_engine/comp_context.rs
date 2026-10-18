use super::*;

/// Snapshot of a layer's transform at a frame, exposed to Rhai as a custom type.
#[derive(Clone, Debug)]
pub struct MaskSnapshot {
    pub mask_shape: Vec<[f64; 2]>,
    pub mask_opacity: f64,
    pub mask_feather: f64,
    pub mask_inverted: bool,
}

#[derive(Clone, Debug)]
pub struct LayerSnapshot {
    pub position: [f64; 2],
    pub scale: [f64; 2],
    pub rotation: f64,
    pub opacity: f64,
    pub anchor_point: [f64; 2],
    pub rotation_3d: [f64; 3],
    pub position_3d: [f64; 3],
    pub time_remap: Option<f64>,
    pub start_time: f64,
    pub stretch: f64,
    pub masks: std::collections::HashMap<String, MaskSnapshot>,
    /// Effect name -> param label -> evaluated value at the snapshot frame.
    /// Vec2 params are stored as "X"/"Y" entries.
    pub effects: std::collections::HashMap<String, std::collections::HashMap<String, f64>>,
}

/// Snapshot of the whole composition, exposed to Rhai as `thisComp`.
#[derive(Clone)]
pub struct CompSnapshot {
    pub layers: std::collections::HashMap<String, LayerSnapshot>,
    pub comp_width: f64,
    pub comp_height: f64,
    /// Layer names in composition stacking order (index 0 = bottom),
    /// enabling AE-style 1-based `thisComp.layer(n)` lookups.
    pub layer_order: Vec<String>,
    /// Composition length in frames.
    pub duration_frames: f64,
    /// Composition frame rate.
    pub fps: f64,
}

impl CompSnapshot {
    /// AE-style `thisComp.layer("Name")` — looks up by name or id.
    pub fn layer(&self, name: &str) -> LayerSnapshot {
        self.layers.get(name).cloned().unwrap_or(LayerSnapshot {
            position: [0.0; 2],
            scale: [100.0, 100.0],
            rotation: 0.0,
            opacity: 100.0,
            anchor_point: [0.0; 2],
            rotation_3d: [0.0; 3],
            position_3d: [0.0; 3],
            time_remap: None,
            start_time: 0.0,
            stretch: 1.0,
            masks: Default::default(),
            effects: Default::default(),
        })
    }

    /// AE-style `thisComp.layer(n)` — 1-based stack order (topmost = 1).
    pub fn layer_by_index(&self, index: i64) -> LayerSnapshot {
        let n = self.layer_order.len() as i64;
        if index < 1 || index > n {
            return self.layer("");
        }
        self.layer(&self.layer_order[(n - index) as usize])
    }

    /// Number of layers in the composition.
    pub fn num_layers(&self) -> f64 {
        self.layer_order.len() as f64
    }

    /// Composition dimensions in pixels.
    pub fn width(&self) -> f64 {
        self.comp_width
    }
    pub fn height(&self) -> f64 {
        self.comp_height
    }

    /// Total duration in seconds (AE `thisComp.duration`).
    pub fn duration(&self) -> f64 {
        if self.fps > 0.0 {
            self.duration_frames / self.fps
        } else {
            0.0
        }
    }

    /// Seconds per frame (AE `thisComp.frameDuration`).
    pub fn frame_duration(&self) -> f64 {
        if self.fps > 0.0 {
            1.0 / self.fps
        } else {
            0.0
        }
    }
}

thread_local! {
    /// Transform of the layer currently being evaluated, for fromComp/toComp.
    /// (position px, scale %, rotation deg, 1-based stack index)
    static CURRENT_LAYER_XFORM: std::cell::RefCell<([f64; 2], [f64; 2], f64)> =
        const { std::cell::RefCell::new(([0.0, 0.0], [100.0, 100.0], 0.0)) };
}

/// Set the per-evaluation layer context used by `toComp` / `fromComp`.
pub fn set_current_layer_xform(pos: [f32; 2], scale: [f32; 2], rotation_deg: f32) {
    CURRENT_LAYER_XFORM.with(|c| {
        *c.borrow_mut() = (
            [pos[0] as f64, pos[1] as f64],
            [scale[0] as f64, scale[1] as f64],
            rotation_deg as f64,
        )
    });
}

fn with_current_xform<T>(f: impl FnOnce([f64; 2], [f64; 2], f64) -> T) -> T {
    CURRENT_LAYER_XFORM.with(|c| {
        let (p, s, r) = *c.borrow();
        f(p, s, r)
    })
}

fn register_comp_types(engine: &mut Engine) {
    engine
        .register_type::<LayerSnapshot>()
        .register_get("transform", |l: &mut LayerSnapshot| l.clone())
        .register_get("position", |l: &mut LayerSnapshot| -> Array {
            vec![
                Dynamic::from_float(l.position[0]),
                Dynamic::from_float(l.position[1]),
            ]
        })
        .register_get("scale", |l: &mut LayerSnapshot| -> Array {
            vec![
                Dynamic::from_float(l.scale[0]),
                Dynamic::from_float(l.scale[1]),
            ]
        })
        .register_get("rotation", |l: &mut LayerSnapshot| l.rotation)
        .register_get("opacity", |l: &mut LayerSnapshot| l.opacity)
        .register_get("anchorPoint", |l: &mut LayerSnapshot| -> Array {
            vec![
                Dynamic::from_float(l.anchor_point[0]),
                Dynamic::from_float(l.anchor_point[1]),
            ]
        })
        .register_get("rotationX", |l: &mut LayerSnapshot| l.rotation_3d[0])
        .register_get("rotationY", |l: &mut LayerSnapshot| l.rotation_3d[1])
        .register_get("rotationZ", |l: &mut LayerSnapshot| l.rotation_3d[2])
        .register_get("orientation", |l: &mut LayerSnapshot| -> Array {
            vec![
                Dynamic::from_float(l.rotation_3d[0]),
                Dynamic::from_float(l.rotation_3d[1]),
                Dynamic::from_float(l.rotation_3d[2]),
            ]
        })
        .register_get("timeRemap", |l: &mut LayerSnapshot| {
            l.time_remap.unwrap_or(0.0)
        })
        .register_get("startTime", |l: &mut LayerSnapshot| l.start_time)
        .register_get("stretch", |l: &mut LayerSnapshot| l.stretch)
        .register_indexer_get(|l: &mut LayerSnapshot, idx: i64| -> Dynamic {
            let arr = [l.position[0], l.position[1]];
            match idx {
                0 => Dynamic::from_float(arr[0]),
                1 => Dynamic::from_float(arr[1]),
                _ => Dynamic::UNIT,
            }
        });

    engine
        .register_type::<CompSnapshot>()
        .register_fn("layer", |c: &mut CompSnapshot, name: &str| c.layer(name))
        .register_fn("layer", |c: &mut CompSnapshot, index: i64| {
            c.layer_by_index(index)
        })
        .register_fn("numLayers", |c: &mut CompSnapshot| c.num_layers())
        .register_fn("duration", |c: &mut CompSnapshot| c.duration())
        .register_fn("frameDuration", |c: &mut CompSnapshot| c.frame_duration());

    engine.register_fn(
        "effect_param",
        |l: &mut LayerSnapshot, effect: &str, param: &str| -> f64 {
            l.effects
                .get(effect)
                .and_then(|m| m.get(param))
                .copied()
                .unwrap_or(f64::NAN)
        },
    );

    // ── AE spatial transforms (thisLayer context) ──
    // toComp(layerPoint): layer-space px → comp-space px
    engine.register_fn("toComp", |x: f64, y: f64| -> Array {
        with_current_xform(|pos, scale, rot| {
            let rad = rot.to_radians();
            let sx = x * scale[0] / 100.0;
            let sy = y * scale[1] / 100.0;
            let cx = sx * rad.cos() - sy * rad.sin();
            let cy = sx * rad.sin() + sy * rad.cos();
            vec![
                Dynamic::from_float(pos[0] + cx),
                Dynamic::from_float(pos[1] + cy),
            ]
        })
    });
    // fromComp(compPoint): comp-space px → layer-space px
    engine.register_fn("fromComp", |x: f64, y: f64| -> Array {
        with_current_xform(|pos, scale, rot| {
            let dx = x - pos[0];
            let dy = y - pos[1];
            let rad = -rot.to_radians();
            let rx = dx * rad.cos() - dy * rad.sin();
            let ry = dx * rad.sin() + dy * rad.cos();
            vec![
                Dynamic::from_float(rx * 100.0 / scale[0].abs().max(0.0001)),
                Dynamic::from_float(ry * 100.0 / scale[1].abs().max(0.0001)),
            ]
        })
    });
}

thread_local! {
    // Single-slot memo: building a snapshot is O(layers); expression layers each
    // need one per frame, which is O(n^2) across a comp without this cache.
    // Keyed by (project version, frame, comp identity) — any edit bumps the
    // global version, so staleness is impossible.
    static SNAPSHOT_CACHE: std::cell::RefCell<Option<(u64, u32, usize, CompSnapshot)>> =
        const { std::cell::RefCell::new(None) };
}

/// Builds a composition snapshot of all layer transforms at the given frame.
pub fn build_comp_snapshot(comp: &crate::core::timeline::Composition, frame: u32) -> CompSnapshot {
    let ver = crate::core::frame_cache::current_version();
    let comp_id = comp as *const _ as usize;
    if let Some(snap) = SNAPSHOT_CACHE.with(|c| {
        c.borrow().as_ref().and_then(|(v, f, cid, snap)| {
            (*v == ver && *f == frame && *cid == comp_id).then(|| snap.clone())
        })
    }) {
        return snap;
    }
    let snapshot = build_comp_snapshot_uncached(comp, frame);
    SNAPSHOT_CACHE.with(|c| *c.borrow_mut() = Some((ver, frame, comp_id, snapshot.clone())));
    snapshot
}

fn build_comp_snapshot_uncached(
    comp: &crate::core::timeline::Composition,
    frame: u32,
) -> CompSnapshot {
    let fps = comp.fps;
    let mut layers = std::collections::HashMap::new();
    for l in &comp.layers {
        let mut fx_map: std::collections::HashMap<String, std::collections::HashMap<String, f64>> =
            std::collections::HashMap::new();
        if l.effects_enabled {
            for eff in &l.effects {
                if !eff.enabled {
                    continue;
                }
                let mut params = std::collections::HashMap::new();
                for (label, pref) in eff.effect_type.animatable_params_ref() {
                    match pref {
                        crate::core::effect_params::ParamRefRef::Scalar(a) => {
                            params.insert(label.to_string(), a.evaluate(frame) as f64);
                        }
                        crate::core::effect_params::ParamRefRef::Vec2(a) => {
                            let v = a.evaluate(frame);
                            params.insert(format!("{} X", label), v[0] as f64);
                            params.insert(format!("{} Y", label), v[1] as f64);
                            params.insert(label.to_string(), v[0] as f64);
                        }
                        crate::core::effect_params::ParamRefRef::Vec3(a) => {
                            let v = a.evaluate(frame);
                            params.insert(format!("{} X", label), v[0] as f64);
                            params.insert(format!("{} Y", label), v[1] as f64);
                            params.insert(format!("{} Z", label), v[2] as f64);
                        }
                        crate::core::effect_params::ParamRefRef::Vec4Color(a) => {
                            let v = a.evaluate(frame);
                            params.insert(label.to_string(), (v[0] + v[1] + v[2]).max(v[3]) as f64);
                        }
                    }
                }
                fx_map.insert(eff.name.clone(), params);
            }
        }
        let pos3d = l.transform_3d.position.evaluate(frame);
        let rot3d = l.transform_3d.rotation.evaluate(frame);
        let anchor3d = [0.0, 0.0, 0.0];
        let time_remap_f = l
            .time_remap
            .as_ref()
            .map(|t| t.evaluate(frame) as f64)
            .unwrap_or(0.0);
        let snap = LayerSnapshot {
            position: [
                l.transform.eval_position(frame, fps)[0] as f64,
                l.transform.eval_position(frame, fps)[1] as f64,
            ],
            scale: [
                l.transform.eval_scale(frame, fps)[0] as f64,
                l.transform.eval_scale(frame, fps)[1] as f64,
            ],
            rotation: l.transform.eval_rotation(frame, fps) as f64,
            opacity: l.transform.eval_opacity(frame, fps) as f64,
            anchor_point: [anchor3d[0], anchor3d[1]],
            rotation_3d: [rot3d[0] as f64, rot3d[1] as f64, rot3d[2] as f64],
            position_3d: [pos3d[0] as f64, pos3d[1] as f64, pos3d[2] as f64],
            time_remap: Some(time_remap_f),
            start_time: l.in_frame as f64,
            stretch: 1.0,
            masks: std::collections::HashMap::new(),
            effects: fx_map,
        };
        layers.insert(l.name.clone(), snap.clone());
        if l.id != l.name && !layers.contains_key(&l.id) {
            layers.insert(l.id.clone(), snap);
        }
    }
    CompSnapshot {
        layers,
        comp_width: comp.width as f64,
        comp_height: comp.height as f64,
        layer_order: comp.layers.iter().map(|l| l.name.clone()).collect(),
        duration_frames: comp.duration_frames as f64,
        fps: comp.fps as f64,
    }
}

/// Evaluate a v2 expression with composition context (thisComp / thisLayer).
pub fn eval_v2_with_comp(
    script: &str,
    base: [f32; 2],
    frame: u32,
    fps: u32,
    comp_snap: &CompSnapshot,
    this_layer: Option<&LayerSnapshot>,
) -> [f32; 2] {
    COMP_ENGINE.with(|engine| {
        let time = frame as f64 / fps.max(1) as f64;
        set_current_time(time);
        // Seed the spatial-transform context for fromComp/toComp
        if let Some(l) = this_layer {
            set_current_layer_xform(
                [l.position[0] as f32, l.position[1] as f32],
                [l.scale[0] as f32, l.scale[1] as f32],
                l.rotation as f32,
            );
        }
        let mut scope = Scope::new();
        scope.push("time", time);
        scope.push("frame", frame as i64);
        scope.push("fps", fps as i64);
        scope.push("index", 0i64); // layer index (threaded at call sites later)
        let base_arr: Array = vec![
            Dynamic::from_float(base[0] as f64),
            Dynamic::from_float(base[1] as f64),
        ];
        scope.push("value", base_arr);
        scope.push("thisComp", comp_snap.clone());
        if let Some(tl) = this_layer {
            scope.push("thisLayer", tl.clone());
        }

        match engine.eval_with_scope::<Dynamic>(&mut scope, script) {
            Ok(val) => {
                if let Ok(arr) = val.clone().into_array() {
                    if arr.len() >= 2 {
                        return [
                            arr[0]
                                .as_float()
                                .map(|value| finite_or_fallback(value, base[0]))
                                .unwrap_or(base[0]),
                            arr[1]
                                .as_float()
                                .map(|value| finite_or_fallback(value, base[1]))
                                .unwrap_or(base[1]),
                        ];
                    }
                }
                if let Ok(f) = val.as_float() {
                    let x = finite_or_fallback(f, 0.0) + base[0];
                    return [if x.is_finite() { x } else { base[0] }, base[1]];
                }
                base
            }
            Err(e) => {
                log::warn!("[ExprEngine] eval_v2_with_comp error: {}", e);
                base
            }
        }
    })
}

pub fn eval_v2_with_comp_and_property(
    script: &str,
    base: [f32; 2],
    frame: u32,
    fps: u32,
    comp_snap: &CompSnapshot,
    this_layer: Option<&LayerSnapshot>,
    source: &Animatable<[f32; 2]>,
) -> [f32; 2] {
    let source = (script.contains("valueAtTime") || script.contains("velocityAtTime"))
        .then(|| source.clone());
    with_vector_value_at_time_context(base, source, fps, || {
        eval_v2_with_comp(script, base, frame, fps, comp_snap, this_layer)
    })
}

/// Evaluate a scalar expression with composition context.
pub fn eval_f32_with_comp(
    script: &str,
    base: f32,
    frame: u32,
    fps: u32,
    comp_snap: &CompSnapshot,
    this_layer: Option<&LayerSnapshot>,
) -> f32 {
    eval_f32_with_comp_source(script, base, frame, fps, comp_snap, this_layer, None)
}

pub fn eval_f32_with_comp_and_property(
    script: &str,
    base: f32,
    frame: u32,
    fps: u32,
    comp_snap: &CompSnapshot,
    this_layer: Option<&LayerSnapshot>,
    source: &Animatable<f32>,
) -> f32 {
    eval_f32_with_comp_source(
        script,
        base,
        frame,
        fps,
        comp_snap,
        this_layer,
        Some(source),
    )
}

fn eval_f32_with_comp_source(
    script: &str,
    base: f32,
    frame: u32,
    fps: u32,
    comp_snap: &CompSnapshot,
    this_layer: Option<&LayerSnapshot>,
    source: Option<&Animatable<f32>>,
) -> f32 {
    let source =
        source.filter(|_| script.contains("valueAtTime") || script.contains("velocityAtTime"));
    let source = source.cloned();
    with_value_at_time_context(base, source, fps, || {
        COMP_ENGINE.with(|engine| {
            let time = frame as f64 / fps.max(1) as f64;
            set_current_time(time);
            let mut scope = Scope::new();
            scope.push("time", time);
            scope.push("frame", frame as i64);
            scope.push("fps", fps as i64);
            scope.push("index", 0i64); // layer index (threaded at call sites later)
            scope.push("value", base as f64);
            scope.push("thisComp", comp_snap.clone());
            if let Some(tl) = this_layer {
                scope.push("thisLayer", tl.clone());
            }

            match engine.eval_with_scope::<Dynamic>(&mut scope, script) {
                Ok(val) => {
                    if let Some(f) = dynamic_to_f64(&val) {
                        return finite_or_fallback(f, base);
                    }
                    if let Ok(i) = val.as_int() {
                        return finite_or_fallback(i as f64, base);
                    }
                    base
                }
                Err(e) => {
                    log::warn!("[ExprEngine] eval_f32_with_comp error: {}", e);
                    base
                }
            }
        })
    })
}

thread_local! {
    pub(super) static COMP_ENGINE: Engine = {
        let mut e = build_engine();
        register_comp_types(&mut e);
        e
    };

    /// Cached engine for loop-enabled expressions with array `+` operator.
    pub(super) static LOOP_ENGINE: Engine = {
        let mut e = build_engine();
        // AE semantics: [x, y] + [x2, y2] is element-wise (Rhai default concatenates)
        e.register_fn("+", |a: Array, b: Array| -> Array {
            let n = a.len().max(b.len());
            (0..n).map(|i| {
                let av = a.get(i).and_then(|d| d.as_float().ok()).unwrap_or(0.0);
                let bv = b.get(i).and_then(|d| d.as_float().ok()).unwrap_or(0.0);
                Dynamic::from_float(av + bv)
            }).collect()
        });
        e
    };
}
