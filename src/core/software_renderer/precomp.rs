pub mod content;
pub mod paint;
pub mod raster;
pub mod render_layer;
pub mod revision;

use super::{render_frame_to_pixels_filtered, rgba_buffer_size};
use crate::core::timeline::Composition;
use render_layer::render_precomp_layers_inner;
use revision::composition_content_revision;
use std::sync::Arc;
use std::sync::{Mutex, OnceLock};

/// Maximum pre-comp nesting depth before we bail out (cycle / pathological nesting guard).
pub const MAX_PRECOMP_DEPTH: u32 = 16;

thread_local! {
    static PRECOMP_DEPTH: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    /// Stack of composition IDs currently being rendered (cycle detection).
    static PRECOMP_STACK: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
    /// Source-layer effects can reference another layer which references a
    /// third one, but a cycle must not recurse forever.
    static SOURCE_LAYER_RENDER_DEPTH: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

static PRECOMP_RENDER_CACHE: OnceLock<Mutex<crate::core::precomp_cache::PrecompCache>> =
    OnceLock::new();

fn with_precomp_cache<R>(f: impl FnOnce(&mut crate::core::precomp_cache::PrecompCache) -> R) -> R {
    let cache = PRECOMP_RENDER_CACHE
        .get_or_init(|| Mutex::new(crate::core::precomp_cache::PrecompCache::new(64)));
    let mut guard = cache
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    f(&mut guard)
}

const MAX_SOURCE_LAYER_RENDER_DEPTH: u32 = 16;

struct PrecompRenderGuard {
    previous_depth: u32,
}

impl Drop for PrecompRenderGuard {
    fn drop(&mut self) {
        PRECOMP_DEPTH.with(|depth| depth.set(self.previous_depth));
        PRECOMP_STACK.with(|stack| {
            stack.borrow_mut().pop();
        });
    }
}

struct SourceLayerRenderDepthGuard(u32);

impl Drop for SourceLayerRenderDepthGuard {
    fn drop(&mut self) {
        SOURCE_LAYER_RENDER_DEPTH.with(|depth| depth.set(self.0));
    }
}

/// Render a pre-comp by recursively rendering its layers into a pixel buffer.
/// This is the core of pre-comp nesting support. Uses a thread-local cache
/// to avoid redundant re-renders when the same pre-comp is referenced by
/// multiple layers at the same frame.
pub fn render_precomp_layers(
    _comp: &Composition,
    precomp_comp: &Composition,
    frame: u32,
    width: u32,
    height: u32,
) -> Arc<[u8]> {
    let allocation_scope = super::RenderAllocationScope::install();
    let content_revision = composition_content_revision(precomp_comp, frame);
    // Cache check: skip full render if we already have this precomp's pixels
    let cached = with_precomp_cache(|cache| {
        cache.get(&precomp_comp.id, content_revision, frame, width, height)
    });
    if let Some(pixels) = cached {
        return pixels;
    }

    // Cycle detection: a comp that (indirectly) contains itself returns empty
    // immediately instead of burning the whole depth budget on garbage.
    let cyclic = PRECOMP_STACK.with(|stack| {
        let mut s = stack.borrow_mut();
        if s.iter().any(|id| id == &precomp_comp.id) {
            true
        } else {
            s.push(precomp_comp.id.clone());
            false
        }
    });
    if cyclic {
        log::warn!(
            "[Renderer] Pre-comp cycle detected at '{}' ; skipping nested render",
            precomp_comp.name
        );
        return Arc::from([]);
    }

    let current_depth = PRECOMP_DEPTH.with(std::cell::Cell::get);
    let _render_guard = PrecompRenderGuard {
        previous_depth: current_depth,
    };
    // Guard against pathologically deep (but acyclic) pre-comp nesting
    if current_depth >= MAX_PRECOMP_DEPTH {
        log::warn!("[Renderer] Pre-comp nesting depth limit exceeded; skipping nested render");
        return Arc::from([]);
    }
    PRECOMP_DEPTH.with(|depth| depth.set(current_depth + 1));
    let result = Arc::<[u8]>::from(render_precomp_layers_inner(
        _comp,
        precomp_comp,
        frame,
        width,
        height,
    ));
    if allocation_scope.failed() {
        return Arc::from([]);
    }
    // Cache the result for future lookups at the same (comp, frame, resolution)
    if !result.is_empty() {
        with_precomp_cache(|cache| {
            cache.insert(
                &precomp_comp.id,
                content_revision,
                frame,
                width,
                height,
                Arc::clone(&result),
            );
        });
    }
    result
}

/// Render a single layer from `comp` at the given frame, returning an RGBA8 buffer
/// of size `width * height * 4`. Used by effects like SetMatte that need another
/// layer's pixel data.
pub(crate) fn render_single_layer_pixels(
    comp: &Composition,
    layer_idx: usize,
    frame: u32,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let size = rgba_buffer_size(width, height).unwrap_or(0);
    if size == 0 {
        return Vec::new();
    }
    if layer_idx >= comp.layers.len() {
        return super::try_zeroed_buffer(size).unwrap_or_default();
    }
    if super::render_cancelled() {
        return super::try_zeroed_buffer(size).unwrap_or_default();
    }

    SOURCE_LAYER_RENDER_DEPTH.with(|depth| {
        let current = depth.get();
        if current >= MAX_SOURCE_LAYER_RENDER_DEPTH {
            log::warn!(
                "[Renderer] source-layer effect recursion exceeded {} levels",
                MAX_SOURCE_LAYER_RENDER_DEPTH
            );
            return super::try_zeroed_buffer(size).unwrap_or_default();
        }

        depth.set(current + 1);
        let _depth_guard = SourceLayerRenderDepthGuard(current);
        render_frame_to_pixels_filtered(comp, frame, width, height, 0.0, 0, Some(layer_idx))
    })
}
