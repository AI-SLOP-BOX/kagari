//! Log sink and project scope management for automation runtime.

use crate::core::timeline::Project;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

pub const MAX_LOG_LINES: usize = 8_192;
pub const MAX_LOG_BYTES: usize = 1024 * 1024;
pub const MAX_SCRIPT_SOURCE_BYTES: usize = 1024 * 1024;

#[derive(Default)]
pub struct LogSink {
    pub lines: Vec<String>,
    pub bytes: usize,
    pub truncated: bool,
}

pub fn append_log(sink: &Arc<Mutex<LogSink>>, message: &str) {
    let Ok(mut sink) = sink.lock() else {
        return;
    };
    if sink.truncated {
        return;
    }
    if sink.lines.len() >= MAX_LOG_LINES || sink.bytes.saturating_add(message.len()) > MAX_LOG_BYTES
    {
        sink.lines.push("[automation log truncated]".to_string());
        sink.truncated = true;
        return;
    }
    sink.bytes = sink.bytes.saturating_add(message.len());
    sink.lines.push(message.to_string());
}

thread_local! {
    /// Project currently being scripted (valid only inside run_script).
    static CURRENT_PROJECT: RefCell<*mut Project> = const { RefCell::new(std::ptr::null_mut()) };
}

#[derive(Debug)]
pub struct ProjectScope;

impl ProjectScope {
    pub fn enter(project: &mut Project) -> Result<Self, String> {
        CURRENT_PROJECT.with(|current| {
            let mut ptr = current.borrow_mut();
            if !ptr.is_null() {
                return Err("nested automation execution is not supported".to_string());
            }
            *ptr = project as *mut Project;
            Ok(Self)
        })
    }
}

impl Drop for ProjectScope {
    fn drop(&mut self) {
        CURRENT_PROJECT.with(|current| {
            *current.borrow_mut() = std::ptr::null_mut();
        });
    }
}

pub fn with_project<R>(f: impl FnOnce(&mut Project) -> R) -> Option<R> {
    CURRENT_PROJECT.with(|p| {
        let ptr = *p.borrow();
        if ptr.is_null() {
            None
        } else {
            Some(f(unsafe { &mut *ptr }))
        }
    })
}
