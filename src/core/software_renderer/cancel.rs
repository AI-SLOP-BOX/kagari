//! Cooperative cancellation for software-renderer work on the current thread.
//!
//! A shared atomic flag lets exports and deadline watchdogs stop long renders;
//! thread-local installation prevents unrelated renders from cancelling each other.

thread_local! {
    static RENDER_CANCEL: std::cell::RefCell<Option<std::sync::Arc<std::sync::atomic::AtomicBool>>> =
        const { std::cell::RefCell::new(None) };
}

/// Installs a cancellation flag for renders on the current thread.
/// Pass `None` to clear. Returns true if cancellation was requested during render.
pub fn set_render_cancel_flag(flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>) {
    RENDER_CANCEL.with(|f| *f.borrow_mut() = flag);
}

pub(crate) struct RenderCancelFlagGuard {
    previous: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
}

impl RenderCancelFlagGuard {
    pub(crate) fn install(flag: std::sync::Arc<std::sync::atomic::AtomicBool>) -> Self {
        let previous = get_render_cancel_flag();
        set_render_cancel_flag(Some(flag));
        Self { previous }
    }
}

impl Drop for RenderCancelFlagGuard {
    fn drop(&mut self) {
        set_render_cancel_flag(self.previous.take());
    }
}

/// Run a render with a cooperative cancellation flag installed for the
/// current thread. The previous flag is restored even if the render panics.
pub fn with_render_cancel_flag<T>(
    flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    render: impl FnOnce() -> T,
) -> T {
    let _guard = RenderCancelFlagGuard::install(flag);
    render()
}

/// Retrieves a clone of the currently installed cancellation flag, if any.
pub fn get_render_cancel_flag() -> Option<std::sync::Arc<std::sync::atomic::AtomicBool>> {
    RENDER_CANCEL.with(|f| f.borrow().clone())
}

/// True if the installed flag has been set (or no flag — never cancelled).
#[inline]
pub(crate) fn render_cancelled() -> bool {
    RENDER_CANCEL.with(|f| {
        f.borrow()
            .as_ref()
            .is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Relaxed))
    })
}
