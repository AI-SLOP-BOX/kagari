use super::*;

mod deterministic;
mod interpolation;
#[path = "registration/spatial.rs"]
mod spatial;
mod temporal;
mod vector;
use spatial::register_spatial_functions;

pub fn build_engine() -> Engine {
    let mut engine = Engine::new();
    engine.set_max_expr_depths(64, 32);
    engine.set_max_operations(100_000);
    engine.set_max_string_size(4096);
    engine.set_max_array_size(1024);
    engine.set_max_modules(0);

    // ── Security: disable dangerous Rhai builtins ──
    engine.disable_symbol("eval");
    engine.disable_symbol("call");
    engine.disable_symbol("import");
    engine.disable_symbol("export");
    engine.disable_symbol("to_json");
    engine.disable_symbol("from_json");
    register_audio_functions(&mut engine);

    // --- wiggle(frequency: f32, amplitude: f32) -> f32 ---
    // Built-in pseudo-random noise matching AE's wiggle expression.
    // Canonical 2-arg AE form: wiggles BOTH dimensions independently using the
    // engine-injected current time; returns an Array for position properties.
    engine.register_fn("wiggle", |freq: f64, amp: f64| -> Array {
        let t = current_time() * freq.max(1e-4) * std::f64::consts::TAU;
        let noise = |seed: f64| -> f64 {
            let n = (t + seed).sin() * 0.70
                + ((t + seed) * 2.1_f64).sin() * 0.20
                + ((t + seed) * 5.3_f64).sin() * 0.10;
            n * amp
        };
        vec![
            Dynamic::from_float(noise(0.0)),
            Dynamic::from_float(noise(17.0)),
        ]
    });
    // Legacy/explicit 3-arg form kept for existing projects.
    engine.register_fn("wiggle", |time: f64, freq: f64, amp: f64| -> f64 {
        let t = time * freq * std::f64::consts::TAU;
        let n = t.sin() * 0.70 + (t * 2.1_f64).sin() * 0.20 + (t * 5.3_f64).sin() * 0.10;
        n * amp
    });

    // ── AE `Math` namespace (Math.sin / Math.PI / Math.pow ...) ──
    #[export_module]
    mod math_ns {
        pub const PI: f64 = std::f64::consts::PI;
        pub const E: f64 = std::f64::consts::E;
        pub const TAU: f64 = std::f64::consts::TAU;
        pub fn sin(x: f64) -> f64 {
            x.sin()
        }
        pub fn cos(x: f64) -> f64 {
            x.cos()
        }
        pub fn tan(x: f64) -> f64 {
            x.tan()
        }
        pub fn asin(x: f64) -> f64 {
            x.asin()
        }
        pub fn acos(x: f64) -> f64 {
            x.acos()
        }
        pub fn atan(x: f64) -> f64 {
            x.atan()
        }
        pub fn atan2(y: f64, x: f64) -> f64 {
            y.atan2(x)
        }
        pub fn abs(x: f64) -> f64 {
            x.abs()
        }
        pub fn floor(x: f64) -> f64 {
            x.floor()
        }
        pub fn ceil(x: f64) -> f64 {
            x.ceil()
        }
        pub fn round(x: f64) -> f64 {
            x.round()
        }
        pub fn sqrt(x: f64) -> f64 {
            x.sqrt()
        }
        pub fn log(x: f64) -> f64 {
            x.ln()
        }
        pub fn log10(x: f64) -> f64 {
            x.log10()
        }
        pub fn exp(x: f64) -> f64 {
            x.exp()
        }
        pub fn pow(b: f64, e: f64) -> f64 {
            b.powf(e)
        }
        pub fn min(a: f64, b: f64) -> f64 {
            a.min(b)
        }
        pub fn max(a: f64, b: f64) -> f64 {
            a.max(b)
        }
    }
    engine.register_static_module("Math", exported_module!(math_ns).into());

    interpolation::register(&mut engine);
    vector::register(&mut engine);
    deterministic::register(&mut engine);
    temporal::register(&mut engine);
    register_spatial_functions(&mut engine);
    engine
}
