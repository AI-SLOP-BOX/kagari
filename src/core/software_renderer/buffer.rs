thread_local! {
    static RENDER_ALLOCATION_DEPTH: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    static RENDER_ALLOCATION_FAILED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub(crate) struct RenderAllocationScope {
    root: bool,
    previous_failure: bool,
}

impl RenderAllocationScope {
    pub(crate) fn install() -> Self {
        let depth = RENDER_ALLOCATION_DEPTH.with(|depth| depth.get());
        let root = depth == 0;
        let previous_failure = if root {
            RENDER_ALLOCATION_FAILED.with(|failed| failed.replace(false))
        } else {
            false
        };
        RENDER_ALLOCATION_DEPTH.with(|depth| depth.set(depth.get().saturating_add(1)));
        Self {
            root,
            previous_failure,
        }
    }

    pub(crate) fn failed(&self) -> bool {
        RENDER_ALLOCATION_FAILED.with(std::cell::Cell::get)
    }
}

impl Drop for RenderAllocationScope {
    fn drop(&mut self) {
        RENDER_ALLOCATION_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
        if self.root {
            RENDER_ALLOCATION_FAILED.with(|failed| failed.set(self.previous_failure));
        }
    }
}

fn mark_allocation_failed() {
    RENDER_ALLOCATION_FAILED.with(|failed| failed.set(true));
}

pub(crate) fn mark_render_allocation_failed() {
    mark_allocation_failed();
}

pub(crate) fn try_zeroed_buffer(size: usize) -> Option<Vec<u8>> {
    let mut buffer = Vec::new();
    if buffer.try_reserve_exact(size).is_err() {
        mark_allocation_failed();
        return None;
    }
    buffer.resize(size, 0);
    Some(buffer)
}

pub(crate) fn try_clone_buffer(source: &[u8]) -> Option<Vec<u8>> {
    let mut buffer = Vec::new();
    if buffer.try_reserve_exact(source.len()).is_err() {
        mark_allocation_failed();
        return None;
    }
    buffer.extend_from_slice(source);
    Some(buffer)
}

pub(crate) fn try_vec_with_capacity<T>(capacity: usize) -> Option<Vec<T>> {
    let mut values = Vec::new();
    if values.try_reserve_exact(capacity).is_err() {
        mark_allocation_failed();
        return None;
    }
    Some(values)
}
