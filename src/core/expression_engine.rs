use crate::core::property::Animatable;
use rhai::{export_module, exported_module};
/// Rhai-powered AE-compatible expression evaluation engine.
///
/// Supports AE-compatible expression APIs:
///   - `time` (current time in seconds)
///   - `wiggle(freq, amp)` → f32 noise offset
///   - `loopOut("cycle")` / `loopOut("pingpong")`
///   - `thisComp.layer("Name").transform.position[0]`  (inter-layer reference)
///   - Any arbitrary Rhai script that returns a number or array.
use rhai::{Array, Dynamic, Engine, Scope};

mod comp_context;
mod loops;
pub use comp_context::*;
pub use loops::*;

/// Per-frame audio data for expression functions (audioAmplitude, audioSpectrum).
/// Set by the renderer before evaluating expressions each frame.
pub struct AudioExprData {
    /// Overall amplitude 0..1 (from audio mixer).
    pub amplitude: f32,
    /// Frequency band amplitudes: [bass, low-mid, mid, high-mid, treble] each 0..1.
    pub bands: [f32; 5],
}

impl Default for AudioExprData {
    fn default() -> Self {
        Self {
            amplitude: 0.0,
            bands: [0.0; 5],
        }
    }
}

/// Extracts a scalar from a Rhai result: numbers directly, or the FIRST
/// element when an Array is returned (AE wiggle returns per-dim arrays).
fn dynamic_to_f64(v: &Dynamic) -> Option<f64> {
    if let Ok(f) = v.as_float() {
        return Some(f);
    }
    if let Ok(i) = v.as_int() {
        return Some(i as f64);
    }
    if let Ok(arr) = v.clone().into_array() {
        if let Some(first) = arr.first() {
            return dynamic_to_f64(first);
        }
    }
    None
}

fn finite_or_fallback(value: f64, fallback: f32) -> f32 {
    let converted = value as f32;
    if converted.is_finite() {
        converted
    } else if fallback.is_finite() {
        fallback
    } else {
        0.0
    }
}

thread_local! {
    static AUDIO_DATA: std::cell::RefCell<AudioExprData> = const { std::cell::RefCell::new(AudioExprData { amplitude: 0.0, bands: [0.0; 5] }) };
}

/// Set the current frame's audio data (call before expression evaluation).
pub fn set_audio_expr_data(data: AudioExprData) {
    AUDIO_DATA.with(|d| *d.borrow_mut() = data);
}

/// Get the current amplitude (0..1).
pub fn get_audio_amplitude() -> f32 {
    AUDIO_DATA.with(|d| d.borrow().amplitude)
}

/// Get a frequency band by index (0=bass, 1=low-mid, 2=mid, 3=high-mid, 4=treble).
pub fn get_audio_band(idx: usize) -> f32 {
    AUDIO_DATA.with(|d| {
        let data = d.borrow();
        if idx < 5 {
            data.bands[idx]
        } else {
            0.0
        }
    })
}

/// Export the current expression audio state using the shared binding names.
/// This keeps GUI preview, expressions, and Audio→VFX automation on one source.
pub fn audio_binding_source_values() -> std::collections::HashMap<String, f64> {
    AUDIO_DATA.with(|data| {
        let data = data.borrow();
        let mut values = std::collections::HashMap::with_capacity(6);
        values.insert("audio.amplitude".into(), f64::from(data.amplitude));
        let names = [
            "audio.bass",
            "audio.low_mid",
            "audio.mid",
            "audio.high_mid",
            "audio.treble",
        ];
        for (index, value) in data.bands.iter().enumerate() {
            values.insert(format!("audio.band{index}"), f64::from(*value));
            values.insert(names[index].into(), f64::from(*value));
        }
        values
    })
}

/// Apply binding-style Audio source values to the expression context.
pub fn set_audio_from_binding_sources(values: &std::collections::HashMap<String, f64>) {
    let amplitude = values
        .get("audio.amplitude")
        .copied()
        .unwrap_or_else(|| values.get("audio.rms").copied().unwrap_or(0.0));
    let names = ["bass", "low_mid", "mid", "high_mid", "treble"];
    let mut bands = [0.0; 5];
    for (index, name) in names.iter().enumerate() {
        bands[index] = values
            .get(&format!("audio.{name}"))
            .or_else(|| values.get(&format!("audio.band{index}")))
            .copied()
            .unwrap_or(0.0) as f32;
    }
    if amplitude.is_finite() && bands.iter().all(|value| value.is_finite()) {
        set_audio_expr_data(AudioExprData {
            amplitude: amplitude.clamp(0.0, 1.0) as f32,
            bands: bands.map(|value| value.clamp(0.0, 1.0)),
        });
    }
}

/// Build the shared Rhai engine with all AE-compatible functions registered.
/// Creating the engine is expensive — do it once and cache it.
/// 64-bit Permuted Congruential Generator (PCG32) hash to produce uniform f64 in [0.0, 1.0).
pub fn pcg32_hash(mut state: u64) -> f64 {
    state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let xorshifted = (((state >> 18) ^ state) >> 27) as u32;
    let rot = (state >> 59) as u32;
    let val = (xorshifted >> rot) | (xorshifted << ((!rot).wrapping_add(1) & 31));
    val as f64 / 4294967296.0
}

thread_local! {
    /// Current composition time (seconds), injected by every eval entry point.
    /// Lets zero-state helpers like the canonical `wiggle(freq, amp)` behave
    /// time-aware without changing their Rhai signatures.
    static CURRENT_TIME: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) };
    static VALUE_AT_TIME_CONTEXT: std::cell::RefCell<Option<ValueAtTimeContext>> = const { std::cell::RefCell::new(None) };
}

#[derive(Clone)]
struct ValueAtTimeContext {
    current_value: f32,
    current_vector: Option<[f32; 2]>,
    source: Option<SampledProperty>,
    fps: f64,
}

#[derive(Clone)]
enum SampledProperty {
    Scalar(Animatable<f32>),
    Vector(Animatable<[f32; 2]>),
}

struct ValueAtTimeGuard {
    previous: Option<ValueAtTimeContext>,
}

impl ValueAtTimeGuard {
    fn install(context: ValueAtTimeContext) -> Self {
        let previous = VALUE_AT_TIME_CONTEXT.with(|slot| slot.replace(Some(context)));
        Self { previous }
    }
}

impl Drop for ValueAtTimeGuard {
    fn drop(&mut self) {
        let previous = self.previous.take();
        VALUE_AT_TIME_CONTEXT.with(|slot| {
            slot.replace(previous);
        });
    }
}

fn sample_value_at_time(context: &ValueAtTimeContext, time: f64) -> f64 {
    if !time.is_finite() {
        return context.current_value as f64;
    }
    if let Some(SampledProperty::Scalar(source)) = &context.source {
        let frame = (time * context.fps) as f32;
        let value = source.value_at_f32(frame);
        if value.is_finite() {
            return value as f64;
        }
    }
    context.current_value as f64
}

fn sample_vector_at_time(context: &ValueAtTimeContext, time: f64) -> [f32; 2] {
    if time.is_finite() {
        if let Some(SampledProperty::Vector(source)) = &context.source {
            let frame = (time * context.fps) as f32;
            let value = source.value_at_f32(frame.max(0.0));
            if value.iter().all(|component| component.is_finite()) {
                return value;
            }
        }
    }
    context.current_vector.unwrap_or([context.current_value; 2])
}

fn with_value_at_time_context<T>(
    base: f32,
    source: Option<Animatable<f32>>,
    fps: u32,
    evaluate: impl FnOnce() -> T,
) -> T {
    let _guard = ValueAtTimeGuard::install(ValueAtTimeContext {
        current_value: if base.is_finite() { base } else { 0.0 },
        current_vector: None,
        source: source.map(SampledProperty::Scalar),
        fps: fps.max(1) as f64,
    });
    evaluate()
}

fn with_vector_value_at_time_context<T>(
    base: [f32; 2],
    source: Option<Animatable<[f32; 2]>>,
    fps: u32,
    evaluate: impl FnOnce() -> T,
) -> T {
    let _guard = ValueAtTimeGuard::install(ValueAtTimeContext {
        current_value: if base[0].is_finite() { base[0] } else { 0.0 },
        current_vector: Some([
            if base[0].is_finite() { base[0] } else { 0.0 },
            if base[1].is_finite() { base[1] } else { 0.0 },
        ]),
        source: source.map(SampledProperty::Vector),
        fps: fps.max(1) as f64,
    });
    evaluate()
}

fn set_current_time(t: f64) {
    CURRENT_TIME.with(|c| c.set(t));
}

#[allow(dead_code)]
pub fn current_time() -> f64 {
    CURRENT_TIME.with(|c| c.get())
}

fn register_audio_functions(engine: &mut Engine) {
    engine.register_fn("audioAmplitude", || -> f64 {
        AUDIO_DATA.with(|data| data.borrow().amplitude as f64)
    });
    engine.register_fn("audioBand", |index: i64| -> f64 {
        AUDIO_DATA.with(|data| {
            let data = data.borrow();
            let index = index.max(0) as usize;
            data.bands.get(index).copied().unwrap_or(0.0) as f64
        })
    });
    engine.register_fn("audioAmplitude", |band_name: &str| -> f64 {
        AUDIO_DATA.with(|data| {
            let data = data.borrow();
            match band_name.to_lowercase().as_str() {
                "bass" | "low" => data.bands[0] as f64,
                "lowmid" | "low-mid" => data.bands[1] as f64,
                "mid" | "midrange" => data.bands[2] as f64,
                "highmid" | "high-mid" => data.bands[3] as f64,
                "treble" | "high" => data.bands[4] as f64,
                _ => data.amplitude as f64,
            }
        })
    });
}

mod registration;
pub use registration::build_engine;

/// Evaluate a Rhai expression that should return a single `f32`.
///
/// # Context variables available in the script
/// - `time`  : current time in seconds (f64)
/// - `frame` : current frame number (i64)
/// - `fps`   : frames per second (i64)
/// - `value` : the un-animated base value (f64)
pub fn eval_f32(engine: &Engine, script: &str, base: f32, frame: u32, fps: u32) -> f32 {
    with_value_at_time_context(base, None, fps, || {
        eval_f32_inner(engine, script, base, frame, fps)
    })
}

pub fn eval_f32_with_property(
    engine: &Engine,
    script: &str,
    base: f32,
    frame: u32,
    fps: u32,
    source: &Animatable<f32>,
) -> f32 {
    let source = script
        .contains("valueAtTime")
        .then(|| source.clone())
        .or_else(|| script.contains("velocityAtTime").then(|| source.clone()));
    with_value_at_time_context(base, source, fps, || {
        eval_f32_inner(engine, script, base, frame, fps)
    })
}

pub fn eval_v2_with_property(
    engine: &Engine,
    script: &str,
    base: [f32; 2],
    frame: u32,
    fps: u32,
    source: &Animatable<[f32; 2]>,
) -> [f32; 2] {
    let source = script
        .contains("valueAtTime")
        .then(|| source.clone())
        .or_else(|| script.contains("velocityAtTime").then(|| source.clone()));
    with_vector_value_at_time_context(base, source, fps, || {
        eval_v2(engine, script, base, frame, fps)
    })
}

fn eval_f32_inner(engine: &Engine, script: &str, base: f32, frame: u32, fps: u32) -> f32 {
    let time = frame as f64 / fps.max(1) as f64;
    set_current_time(time);
    let mut scope = Scope::new();
    scope.push("time", time);
    scope.push("frame", frame as i64);
    scope.push("fps", fps as i64);
    scope.push("index", 0i64); // layer index (threaded at call sites later)
    scope.push("value", base as f64);
    scope.push("comp_width", 1920.0f64);
    scope.push("comp_height", 1080.0f64);

    match engine.eval_with_scope::<Dynamic>(&mut scope, script) {
        Ok(val) => {
            if let Some(f) = dynamic_to_f64(&val) {
                return finite_or_fallback(f, base);
            }
            if let Ok(i) = val.as_int() {
                return finite_or_fallback(i as f64, base);
            }
            log::warn!("[ExprEngine] expression did not return a number: {:?}", val);
            base
        }
        Err(e) => {
            log::warn!("[ExprEngine] eval_f32 error: {}", e);
            base
        }
    }
}

/// Evaluate a Rhai expression that should return a `[f32; 2]` pair.
///
/// The script may return:
///   - A Rhai `Array` of two numbers → `[x, y]`
///   - A single number → applied to both X and Y
pub fn eval_v2(engine: &Engine, script: &str, base: [f32; 2], frame: u32, fps: u32) -> [f32; 2] {
    let time = frame as f64 / fps.max(1) as f64;
    set_current_time(time);
    let mut scope = Scope::new();
    scope.push("time", time);
    scope.push("frame", frame as i64);
    scope.push("fps", fps as i64);
    scope.push("index", 0i64); // layer index (threaded at call sites later)
    scope.push("comp_width", 1920.0f64);
    scope.push("comp_height", 1080.0f64);
    // Expose base value as a Rhai array for scripts like `value + wiggle(...)`
    let base_arr: Array = vec![
        Dynamic::from_float(base[0] as f64),
        Dynamic::from_float(base[1] as f64),
    ];
    scope.push("value", base_arr);

    match engine.eval_with_scope::<Dynamic>(&mut scope, script) {
        Ok(val) => {
            // Array return
            if let Ok(arr) = val.clone().into_array() {
                if arr.len() >= 2 {
                    let x = arr[0]
                        .as_float()
                        .map(|value| finite_or_fallback(value, base[0]))
                        .unwrap_or(base[0]);
                    let y = arr[1]
                        .as_float()
                        .map(|value| finite_or_fallback(value, base[1]))
                        .unwrap_or(base[1]);
                    return [x, y];
                }
            }
            // Scalar return — AE standard applies scalar offset to X axis only
            if let Ok(f) = val.as_float() {
                let x = finite_or_fallback(f, 0.0) + base[0];
                return [if x.is_finite() { x } else { base[0] }, base[1]];
            }
            base
        }
        Err(e) => {
            log::warn!("[ExprEngine] eval_v2 error: {}", e);
            base
        }
    }
}

/// Evaluate expression with detailed diagnostic feedback for UI toast notifications.
#[allow(dead_code)]
pub fn eval_v2_with_diagnostics(
    engine: &Engine,
    script: &str,
    base: [f32; 2],
    frame: u32,
    fps: u32,
) -> ([f32; 2], Option<String>) {
    let time = frame as f64 / fps.max(1) as f64;
    set_current_time(time);
    let mut scope = Scope::new();
    scope.push("time", time);
    scope.push("frame", frame as i64);
    scope.push("fps", fps as i64);
    scope.push("index", 0i64); // layer index (threaded at call sites later)
    scope.push("comp_width", 1920.0f64);
    scope.push("comp_height", 1080.0f64);
    let base_arr: Array = vec![
        Dynamic::from_float(base[0] as f64),
        Dynamic::from_float(base[1] as f64),
    ];
    scope.push("value", base_arr);

    match engine.eval_with_scope::<Dynamic>(&mut scope, script) {
        Ok(val) => {
            if let Ok(arr) = val.clone().into_array() {
                if arr.len() >= 2 {
                    let x = arr[0]
                        .as_float()
                        .map(|value| finite_or_fallback(value, base[0]))
                        .unwrap_or(base[0]);
                    let y = arr[1]
                        .as_float()
                        .map(|value| finite_or_fallback(value, base[1]))
                        .unwrap_or(base[1]);
                    return ([x, y], None);
                }
            }
            if let Ok(f) = val.as_float() {
                let x = finite_or_fallback(f, 0.0) + base[0];
                return ([if x.is_finite() { x } else { base[0] }, base[1]], None);
            }
            (base, Some("Expression returned non-numeric type".into()))
        }
        Err(e) => (base, Some(format!("Script syntax/eval error: {}", e))),
    }
}

/// Pre-validate a script without evaluating (for real-time syntax checking in UI).
/// Returns `Ok(())` if the script compiles, or an error message.
#[allow(dead_code)]
pub fn validate_script(engine: &Engine, script: &str) -> Result<(), String> {
    engine
        .compile(script)
        .map(|_| ())
        .map_err(|e| format!("{}", e))
}

/// Preprocessed loop values exposed to Raw scripts that call loopOut()/loopIn().
#[derive(Clone, Copy, Default)]
pub struct LoopVals {
    pub out_cycle: f32,
    pub out_pingpong: f32,
    pub in_cycle: f32,
    pub in_pingpong: f32,
}

/// Evaluate an expression with arbitrary f64 variables (for text selectors, etc.).
pub fn eval_expression_f64(script: &str, vars: &[(&str, f64)]) -> f64 {
    LOOP_ENGINE.with(|engine| {
        let mut scope = Scope::new();
        for (name, val) in vars {
            scope.push(*name, *val);
        }
        engine
            .eval_with_scope::<f64>(&mut scope, script)
            .ok()
            .filter(|value| value.is_finite())
            .unwrap_or(0.0)
    })
}

// ──────────────── Pick Whip Expression Generator ────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickWhipTarget {
    TransformProperty {
        layer_name: String,
        prop: String,
    },
    EffectProperty {
        layer_name: String,
        effect_name: String,
        param_name: String,
    },
    LayerSelf {
        layer_name: String,
    },
    ExternalCompProperty {
        comp_name: String,
        layer_name: String,
        prop_path: String,
    },
}

/// Automatically generates valid AE-compatible expression code when using the Pick Whip tool.
pub fn generate_pick_whip_expression(target: &PickWhipTarget, current_layer_name: &str) -> String {
    match target {
        PickWhipTarget::TransformProperty { layer_name, prop } => {
            if layer_name == current_layer_name {
                format!("transform.{}", prop)
            } else {
                format!("thisComp.layer(\"{}\").transform.{}", layer_name, prop)
            }
        }
        PickWhipTarget::EffectProperty {
            layer_name,
            effect_name,
            param_name,
        } => {
            if layer_name == current_layer_name {
                format!("effect(\"{}\")(\"{}\")", effect_name, param_name)
            } else {
                format!(
                    "thisComp.layer(\"{}\").effect(\"{}\")(\"{}\")",
                    layer_name, effect_name, param_name
                )
            }
        }
        PickWhipTarget::LayerSelf { layer_name } => {
            if layer_name == current_layer_name {
                "thisLayer".to_string()
            } else {
                format!("thisComp.layer(\"{}\")", layer_name)
            }
        }
        PickWhipTarget::ExternalCompProperty {
            comp_name,
            layer_name,
            prop_path,
        } => {
            format!(
                "comp(\"{}\").layer(\"{}\").{}",
                comp_name, layer_name, prop_path
            )
        }
    }
}

#[cfg(test)]
mod tests;
