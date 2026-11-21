//! Rhai automation engine bindings and project mutating functions.

use super::context::{append_log, with_project, LogSink};
use crate::core::keyframe::{InterpolationType, Keyframe};
use crate::core::property::Animatable;
use crate::core::timeline::{Composition, Layer, LayerType};
use std::sync::{Arc, Mutex};

pub(crate) fn automation_runtime_error(message: impl Into<String>) -> Box<rhai::EvalAltResult> {
    rhai::EvalAltResult::ErrorRuntime(rhai::Dynamic::from(message.into()), rhai::Position::NONE)
        .into()
}

pub(crate) fn ensure_collection_capacity(
    current_len: usize,
    limit: usize,
    description: impl Into<String>,
) -> Result<(), Box<rhai::EvalAltResult>> {
    if current_len >= limit {
        Err(automation_runtime_error(format!(
            "automation cannot add more {}: limit reached",
            description.into()
        )))
    } else {
        Ok(())
    }
}

pub(crate) fn gen_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    format!("script_layer_{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}

pub fn parse_hex(hex: &str) -> [f32; 3] {
    let h = hex.trim_start_matches('#');
    if h.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&h[0..2], 16),
            u8::from_str_radix(&h[2..4], 16),
            u8::from_str_radix(&h[4..6], 16),
        ) {
            return [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0];
        }
    }
    [1.0, 1.0, 1.0]
}

pub(crate) fn finite_f32(value: f64) -> f32 {
    if value.is_finite() {
        value.clamp(f64::from(f32::MIN), f64::from(f32::MAX)) as f32
    } else {
        0.0
    }
}

pub(crate) fn bounded_u32(value: i64, minimum: u32) -> u32 {
    value.clamp(i64::from(minimum), i64::from(u32::MAX)) as u32
}

pub fn build_engine(log_sink: Arc<Mutex<LogSink>>) -> rhai::Engine {
    let mut engine = rhai::Engine::new();
    engine.set_max_operations(2_000_000);
    engine
        .set_max_string_size(1024 * 1024)
        .set_max_array_size(16 * 1024)
        .set_max_map_size(16 * 1024);

    engine.on_print({
        let sink = Arc::clone(&log_sink);
        move |msg: &str| {
            append_log(&sink, msg);
        }
    });

    // ── Composition management ──
    engine.register_fn("log", move |msg: &str| {
        append_log(&log_sink, msg);
    });
    engine.register_fn(
        "new_comp",
        |name: &str, w: i64, ht: i64, fps: i64, dur: i64| -> Result<(), Box<rhai::EvalAltResult>> {
            with_project(|p| {
                ensure_collection_capacity(
                    p.compositions.len(),
                    crate::core::production_document::ProductionDocument::MAX_COMPOSITIONS,
                    "compositions",
                )?;
                let comp = Composition::new(
                    p.next_composition_id("comp"),
                    name.to_string(),
                    bounded_u32(w, 1),
                    bounded_u32(ht, 1),
                    bounded_u32(fps, 1),
                    bounded_u32(dur, 1),
                );
                p.compositions.push(comp);
                p.active_composition_idx = p.compositions.len() - 1;
                Ok(())
            })
            .unwrap_or_else(|| Err(automation_runtime_error("no active project")))
        },
    );
    engine.register_fn("select_comp", |idx: i64| {
        with_project(|p| {
            if idx >= 0 && (idx as usize) < p.compositions.len() {
                p.active_composition_idx = idx as usize;
            }
        });
    });

    // ── Layer creation ──
    engine.register_fn(
        "add_solid",
        |name: &str, color_hex: &str| -> Result<String, Box<rhai::EvalAltResult>> {
            with_project(|p| {
                let Some(idx) = p
                    .compositions
                    .get(p.active_composition_idx)
                    .map(|_| p.active_composition_idx)
                else {
                    return Err(automation_runtime_error(
                        "add_solid requires an active composition",
                    ));
                };
                ensure_collection_capacity(
                    p.compositions[idx].layers.len(),
                    crate::core::production_document::ProductionDocument::MAX_LAYERS_PER_COMPOSITION,
                    format!("layers to composition '{}'", p.compositions[idx].name),
                )?;
                let c = parse_hex(color_hex);
                let comp_len = { p.compositions[idx].duration_frames };
                let mut layer = Layer::new(
                    gen_id(),
                    name.to_string(),
                    LayerType::Solid {
                        color: [c[0], c[1], c[2], 1.0],
                    },
                    comp_len,
                );
                let comp = &mut p.compositions[idx];
                let cw = comp.width as f32;
                let ch = comp.height as f32;
                layer.transform.scale = Animatable::new_constant([cw, ch]);
                layer.transform.position = Animatable::new_constant([cw / 2.0, ch / 2.0]);
                let n = layer.name.clone();
                comp.layers.push(layer);
                Ok(n)
            })
            .unwrap_or_else(|| Err(automation_runtime_error("no active project")))
        },
    );
    engine.register_fn(
        "add_text",
        |name: &str, text: &str, size: i64| -> Result<String, Box<rhai::EvalAltResult>> {
            with_project(|p| {
                let Some(idx) = p
                    .compositions
                    .get(p.active_composition_idx)
                    .map(|_| p.active_composition_idx)
                else {
                    return Err(automation_runtime_error(
                        "add_text requires an active composition",
                    ));
                };
                ensure_collection_capacity(
                    p.compositions[idx].layers.len(),
                    crate::core::production_document::ProductionDocument::MAX_LAYERS_PER_COMPOSITION,
                    format!("layers to composition '{}'", p.compositions[idx].name),
                )?;
                let comp_len = { p.compositions[idx].duration_frames };
                let mut layer = Layer::new(
                    gen_id(),
                    name.to_string(),
                    LayerType::Text {
                        text: text.to_string(),
                        font_size: bounded_u32(size, 4),
                        color: [1.0, 1.0, 1.0, 1.0],
                        font_family: "Inter".to_string(),
                        tracking: 0.0,
                        leading: 1.2,
                        align: 0,
                        stroke_color: [1.0, 1.0, 1.0, 1.0],
                        stroke_width: 0.0,
                        text_on_path: false,
                    },
                    comp_len,
                );
                let comp = &mut p.compositions[idx];
                let cw = comp.width as f32;
                let ch = comp.height as f32;
                layer.transform.scale = Animatable::new_constant([100.0, 100.0]);
                layer.transform.position = Animatable::new_constant([cw / 2.0, ch / 2.0]);
                let n = layer.name.clone();
                comp.layers.push(layer);
                Ok(n)
            })
            .unwrap_or_else(|| Err(automation_runtime_error("no active project")))
        },
    );

    // ── Property animation ──
    engine.register_fn("set_position", |layer: &str, x: f64, y: f64| {
        with_project(|p| {
            let Some(idx) = p
                .compositions
                .get(p.active_composition_idx)
                .map(|_| p.active_composition_idx)
            else {
                return;
            };
            if let Some(i) = p.compositions[idx]
                .layers
                .iter()
                .position(|l| l.name == layer)
            {
                p.compositions[idx].layers[i].transform.position =
                    Animatable::new_constant([finite_f32(x), finite_f32(y)]);
            }
        });
    });
    engine.register_fn("set_opacity", |layer: &str, pct: f64| {
        with_project(|p| {
            let Some(idx) = p
                .compositions
                .get(p.active_composition_idx)
                .map(|_| p.active_composition_idx)
            else {
                return;
            };
            if let Some(i) = p.compositions[idx]
                .layers
                .iter()
                .position(|l| l.name == layer)
            {
                p.compositions[idx].layers[i].transform.opacity =
                    Animatable::new_constant(finite_f32(pct).clamp(0.0, 100.0));
            }
        });
    });
    engine.register_fn("set_opacity", |layer: &str, pct: i64| {
        with_project(|p| {
            let Some(idx) = p
                .compositions
                .get(p.active_composition_idx)
                .map(|_| p.active_composition_idx)
            else {
                return;
            };
            if let Some(i) = p.compositions[idx]
                .layers
                .iter()
                .position(|l| l.name == layer)
            {
                p.compositions[idx].layers[i].transform.opacity =
                    Animatable::new_constant(finite_f32(pct as f64).clamp(0.0, 100.0));
            }
        });
    });
    engine.register_fn("key_position", |layer: &str, frame: i64, x: f64, y: f64| {
        with_project(|p| {
            let Some(idx) = p
                .compositions
                .get(p.active_composition_idx)
                .map(|_| p.active_composition_idx)
            else {
                return;
            };
            let Some(i) = p.compositions[idx]
                .layers
                .iter()
                .position(|l| l.name == layer)
            else {
                return;
            };
            let kf = Keyframe::new(
                bounded_u32(frame, 0),
                [finite_f32(x), finite_f32(y)],
                InterpolationType::Linear,
            );
            match &mut p.compositions[idx].layers[i].transform.position {
                Animatable::Animated(kfs) => {
                    if let Some(existing) =
                        kfs.iter_mut().find(|existing| existing.frame == kf.frame)
                    {
                        *existing = kf;
                    } else {
                        kfs.push(kf);
                    }
                    kfs.sort_by_key(|k| k.frame);
                }
                Animatable::Constant(v0) => {
                    let v = *v0;
                    let keyframes = if kf.frame == 0 {
                        vec![kf]
                    } else {
                        vec![Keyframe::new(0, v, InterpolationType::Linear), kf]
                    };
                    p.compositions[idx].layers[i].transform.position =
                        Animatable::Animated(keyframes);
                }
            }
        });
    });

    // ── Persistence ──
    engine.register_fn("save_project", |path: &str| -> bool {
        with_project(|p| {
            let path = std::path::Path::new(path);
            crate::core::project_migration::save_project_atomic(p, path).is_ok()
        })
        .unwrap_or(false)
    });

    engine
}
