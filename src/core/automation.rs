//! Project automation scripting (Rhai) — the ExtendScript/JSX analogue.
//!
//! Exposes a mutating API over the active project so scripts can batch-create
//! comps, layers, keyframes and save results. Runs identically from the CLI
//! (`kagari script file.rhai`) and from in-app tooling.
//!
//! Implementation note: Rhai evaluation is strictly single-threaded within
//! `run_script`, so a thread-local project pointer + log sink is the simplest
//! sound way to expose mutation. Scripts run against a working copy and are
//! committed only after the whole evaluation succeeds.

#![allow(dead_code)]

pub mod context;
pub mod engine;

#[cfg(test)]
mod tests;

pub use context::*;
pub use engine::*;

use crate::core::timeline::Project;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Execute an automation script against the project.
/// Returns captured `log()` lines on success.
pub fn run_script(project: &mut Project, source: &str) -> Result<Vec<String>, String> {
    let (working_copy, logs) = run_script_working_copy(project.clone(), source, None)?;
    *project = working_copy;
    Ok(logs)
}

/// Run against an owned project snapshot and return the modified snapshot.
/// This avoids cloning the project a second time inside a background worker.
pub fn run_script_owned(
    project: Project,
    source: &str,
    cancel: Arc<AtomicBool>,
) -> Result<(Project, Vec<String>), String> {
    run_script_working_copy(project, source, Some(cancel))
}

fn run_script_working_copy(
    mut working_copy: Project,
    source: &str,
    cancel: Option<Arc<AtomicBool>>,
) -> Result<(Project, Vec<String>), String> {
    if source.len() > MAX_SCRIPT_SOURCE_BYTES {
        return Err(format!(
            "automation script exceeds the {} MiB limit",
            MAX_SCRIPT_SOURCE_BYTES / (1024 * 1024)
        ));
    }
    let log_sink = Arc::new(Mutex::new(LogSink::default()));
    let mut engine = build_engine(Arc::clone(&log_sink));
    if let Some(cancel) = cancel {
        engine.on_progress(move |_| {
            cancel
                .load(Ordering::Relaxed)
                .then(|| "Script cancelled".into())
        });
    }

    let scope = ProjectScope::enter(&mut working_copy)?;
    let result = engine.run(source);
    drop(scope);

    result.map_err(|e| format!("script error: {e}"))?;
    let document = crate::core::production_document::ProductionDocument::new(working_copy);
    document
        .validate()
        .map_err(|error| format!("script produced an invalid project: {error}"))?;
    let logs = log_sink
        .lock()
        .map(|sink| sink.lines.clone())
        .unwrap_or_default();
    Ok((document.project, logs))
}

/// Convenience wrapper executing a `main() { ... }` program body.
pub fn run_script_main(project: &mut Project, body: &str) -> Result<Vec<String>, String> {
    let wrapped = format!("main();\nfn main() {{\n{body}\n}}");
    run_script(project, &wrapped)
}
