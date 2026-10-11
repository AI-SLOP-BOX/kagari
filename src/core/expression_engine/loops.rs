use super::*;

/// Rewrites AE-style loop calls in a Raw script into scope-variable references.
/// `loopOut("cycle")` → `__loop_out_cycle`, `loopOut("pingpong")` → `__loop_out_pingpong`,
/// `loopIn("cycle")` → `__loop_in_cycle`, `loopIn("pingpong")` → `__loop_in_pingpong`,
/// bare `loopOut()` → `__loop_out_cycle`.
pub fn preprocess_loop_script(script: &str) -> String {
    script
        .replace("loopOut(\"pingpong\",", "__loop_out_pingpong")
        .replace("loopOut(\"cycle\",", "__loop_out_cycle")
        .replace("loopIn(\"pingpong\",", "__loop_in_pingpong")
        .replace("loopIn(\"cycle\",", "__loop_in_cycle")
        .replace("loopOut(\"pingpong\")", "__loop_out_pingpong")
        .replace("loopIn(\"pingpong\")", "__loop_in_pingpong")
        .replace("loopOut(\"cycle\")", "__loop_out_cycle")
        .replace("loopIn(\"cycle\")", "__loop_in_cycle")
        .replace("loopOut()", "__loop_out_cycle")
        .replace("loopIn()", "__loop_in_cycle")
        .replace("loopOut", "__loop_out_cycle")
        .replace("loopIn", "__loop_in_cycle")
}

/// True if the script references any loop function.
pub fn script_uses_loops(script: &str) -> bool {
    script.contains("loopOut")
        || script.contains("loopIn")
        || script.contains("loopOutDuration")
        || script.contains("loopInDuration")
}

/// Evaluate a scalar Raw script with loop values available.
pub fn eval_f32_with_loops(script: &str, base: f32, frame: u32, fps: u32, loops: LoopVals) -> f32 {
    let rewritten = preprocess_loop_script(script);
    let time = frame as f64 / fps.max(1) as f64;
    set_current_time(time);
    LOOP_ENGINE.with(|engine| {
        let mut scope = Scope::new();
        scope.push("time", time);
        scope.push("frame", frame as i64);
        scope.push("fps", fps as i64);
        scope.push("index", 0i64); // layer index (threaded at call sites later)
        scope.push("value", base as f64);
        scope.push("__loop_out_cycle", loops.out_cycle as f64);
        scope.push("__loop_out_pingpong", loops.out_pingpong as f64);
        scope.push("__loop_in_cycle", loops.in_cycle as f64);
        scope.push("__loop_in_pingpong", loops.in_pingpong as f64);

        match engine.eval_with_scope::<Dynamic>(&mut scope, &rewritten) {
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
                log::warn!("[ExprEngine] eval_f32_with_loops error: {}", e);
                base
            }
        }
    })
}

/// Evaluate a v2 Raw script with loop values available.
pub fn eval_v2_with_loops(
    script: &str,
    base: [f32; 2],
    frame: u32,
    fps: u32,
    loops: LoopVals,
) -> [f32; 2] {
    let rewritten = preprocess_loop_script(script);
    let time = frame as f64 / fps.max(1) as f64;
    set_current_time(time);
    LOOP_ENGINE.with(|engine| {
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
        let loop_arr =
            |x: f32, y: f32| vec![Dynamic::from_float(x as f64), Dynamic::from_float(y as f64)];
        scope.push(
            "__loop_out_cycle",
            loop_arr(loops.out_cycle, loops.in_cycle),
        );
        scope.push(
            "__loop_out_pingpong",
            loop_arr(loops.out_pingpong, loops.in_pingpong),
        );
        scope.push("__loop_in_cycle", loop_arr(loops.in_cycle, loops.out_cycle));
        scope.push(
            "__loop_in_pingpong",
            loop_arr(loops.in_pingpong, loops.out_pingpong),
        );

        match engine.eval_with_scope::<Dynamic>(&mut scope, &rewritten) {
            Ok(val) => {
                if let Ok(arr) = val.clone().into_array() {
                    if arr.len() >= 2 {
                        return [
                            arr[0].as_float().unwrap_or(base[0] as f64) as f32,
                            arr[1].as_float().unwrap_or(base[1] as f64) as f32,
                        ];
                    }
                }
                if let Ok(f) = val.as_float() {
                    return [base[0] + f as f32, base[1]];
                }
                base
            }
            Err(e) => {
                log::warn!("[ExprEngine] eval_v2_with_loops error: {}", e);
                base
            }
        }
    })
}
