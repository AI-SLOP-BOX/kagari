//! SQLite MVCC-inspired versioned frame render cache.
//!
//! Every time the project changes (a new history commit is made), the cache
//! version increments. Reads always see a consistent snapshot; writes for the
//! new version happen concurrently without invalidating in-progress reads.
//!
//! Includes strict RAM byte-memory limit guards (default 512 MB) & LRU eviction
//! policy to prevent OS Out-Of-Memory (OOM) crashes under 4K/8K video preview.

pub mod disk_cache;
pub mod pool;

use std::collections::HashMap;
use std::sync::Arc;

pub use disk_cache::*;
pub use pool::*;

/// The frame cache. Key is `(frame_index, cache_version)`.
pub struct FrameCache {
    entries: HashMap<(u32, u64), CacheEntry>,
    /// Maximum number of entries before GC triggers automatically.
    max_entries: usize,
    /// Maximum memory limit in bytes for cached RGBA pixel buffers (e.g. 512MB).
    pub max_memory_bytes: usize,
    /// Currently allocated pixel bytes in memory.
    pub current_memory_bytes: usize,
    /// Layer indices that have been modified since last commit.
    dirty_layers: std::collections::HashSet<usize>,
    /// Composition IDs that contain dirty layers.
    dirty_comps: std::collections::HashSet<String>,
    /// Mapping from frame index to the set of layer indices used to render that frame.
    /// Used for partial invalidation: allows reusing previous-version frames when
    /// only some layers have changed.
    frame_layers: std::collections::HashMap<(u32, u64), std::collections::HashSet<usize>>,
    /// When set, this cache ignores the global version counter and uses this
    /// fixed version instead. This prevents parallel tests from polluting each
    /// other's version state.
    version_override: Option<u64>,
    disk_namespace: u128,
}

impl FrameCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_entries,
            max_memory_bytes: 512 * 1024 * 1024, // 512 MB default
            current_memory_bytes: 0,
            dirty_layers: std::collections::HashSet::new(),
            dirty_comps: std::collections::HashSet::new(),
            frame_layers: std::collections::HashMap::new(),
            version_override: None,
            disk_namespace: next_disk_namespace(),
        }
    }

    /// Create a cache with a fixed version for isolated testing.
    /// This bypasses the global version counter entirely, preventing
    /// parallel test pollution.
    pub fn with_version(max_entries: usize, version: u64) -> Self {
        Self {
            entries: HashMap::new(),
            max_entries,
            max_memory_bytes: 512 * 1024 * 1024,
            current_memory_bytes: 0,
            dirty_layers: std::collections::HashSet::new(),
            dirty_comps: std::collections::HashSet::new(),
            frame_layers: std::collections::HashMap::new(),
            version_override: Some(version),
            disk_namespace: next_disk_namespace(),
        }
    }

    /// Return the effective version: either the override or the global counter.
    fn effective_version(&self) -> u64 {
        self.version_override.unwrap_or_else(current_version)
    }

    /// Mark a layer as dirty (modified). Only frames containing this layer need re-rendering.
    pub fn mark_layer_dirty(&mut self, layer_idx: usize) {
        self.dirty_layers.insert(layer_idx);
    }

    /// Mark a composition as dirty (contains modified layers).
    pub fn mark_comp_dirty(&mut self, comp_id: &str) {
        self.dirty_comps.insert(comp_id.to_string());
    }

    /// Mark layers as dirty for partial invalidation.
    pub fn invalidate_layers(&mut self, layer_indices: &[usize]) {
        for &idx in layer_indices {
            self.dirty_layers.insert(idx);
        }
    }

    /// Check if a specific layer is dirty.
    pub fn is_layer_dirty(&self, layer_idx: usize) -> bool {
        self.dirty_layers.contains(&layer_idx)
    }

    /// Check if a frame should be invalidated based on its layer composition.
    /// A frame is dirty if any of the layers used to render it are marked dirty.
    pub fn is_frame_dirty(&self, _frame: u32, layers: &[usize]) -> bool {
        for &layer_idx in layers {
            if self.is_layer_dirty(layer_idx) {
                return true;
            }
        }
        false
    }

    /// Clear all dirty flags (call after a full re-render or commit).
    pub fn clear_dirty(&mut self) {
        self.dirty_layers.clear();
        self.dirty_comps.clear();
    }

    /// Get the set of dirty layer indices.
    pub fn dirty_layers(&self) -> &std::collections::HashSet<usize> {
        &self.dirty_layers
    }

    /// Try to retrieve a cached frame for the current global version (updates LRU timestamp).
    /// Checks RAM first, then falls back to disk cache on miss.
    /// Entries are only valid for the exact current project version.
    pub fn get(&mut self, frame: u32) -> Option<&CacheEntry> {
        self.get_with_layers(frame, &[])
    }

    /// Try to retrieve a cached frame for the current project version.
    /// `frame_layers` is retained as provenance for future content-addressed
    /// invalidation, but never authorizes cross-version reuse by itself.
    pub fn get_with_layers(&mut self, frame: u32, frame_layers: &[usize]) -> Option<&CacheEntry> {
        let ver = self.effective_version();
        let key = (frame, ver);

        // Check if we have a cached entry for this frame in current version
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.lru_stamp = lru_tick();
            return self.entries.get(&key);
        }

        // Check disk cache on miss
        if let Some((pixels, w, h)) = disk_cache::read_frame_in_namespace_with_limit(
            &key,
            self.disk_namespace,
            self.max_memory_bytes,
        ) {
            if pixels.len() > self.max_memory_bytes {
                return None;
            }
            if self.current_memory_bytes.saturating_add(pixels.len()) > self.max_memory_bytes {
                self.collect_garbage();
                if self.current_memory_bytes.saturating_add(pixels.len()) > self.max_memory_bytes {
                    return None;
                }
            }
            self.current_memory_bytes += pixels.len();
            self.entries.insert(
                key,
                CacheEntry {
                    version: ver,
                    width: w,
                    height: h,
                    pixels: Arc::new(pixels),
                    lru_stamp: lru_tick(),
                },
            );
            self.frame_layers
                .insert(key, frame_layers.iter().cloned().collect());
            // Evict from disk once loaded into RAM
            disk_cache::remove_frame_in_namespace(&key, self.disk_namespace);
            return self.entries.get(&key);
        }
        None
    }

    /// Immutable check if frame is cached without updating LRU timestamp.
    pub fn is_cached(&self, frame: u32) -> bool {
        let ver = self.effective_version();
        self.entries.contains_key(&(frame, ver))
    }

    /// Returns the number of cached frames currently stored.
    pub fn cached_count(&self) -> usize {
        self.entries.len()
    }

    /// Store a rendered frame for the current global version.
    pub fn insert(&mut self, frame: u32, width: u32, height: u32, pixels: Vec<u8>) {
        self.insert_with_layers(frame, width, height, pixels, &[]);
    }

    /// Store a rendered frame and record which layers contributed to it.
    /// Layer provenance is metadata only until a complete content-addressed
    /// render key can prove that all other render inputs are unchanged.
    pub fn insert_with_layers(
        &mut self,
        frame: u32,
        width: u32,
        height: u32,
        pixels: Vec<u8>,
        layer_indices: &[usize],
    ) {
        let Some(expected_bytes) = expected_pixel_bytes(width, height) else {
            return;
        };
        if pixels.len() != expected_bytes {
            return;
        }
        let ver = self.effective_version();
        let key = (frame, ver);
        let bytes_size = pixels.len();

        // If replacing existing entry, subtract old size first
        if let Some(old) = self.entries.remove(&key) {
            self.current_memory_bytes = self.current_memory_bytes.saturating_sub(old.pixels.len());
        }
        self.frame_layers.remove(&key);

        self.current_memory_bytes += bytes_size;
        self.entries.insert(
            key,
            CacheEntry {
                version: ver,
                width,
                height,
                pixels: Arc::new(pixels),
                lru_stamp: lru_tick(),
            },
        );

        if !layer_indices.is_empty() {
            self.frame_layers
                .insert(key, layer_indices.iter().copied().collect());
        }

        if self.entries.len() > self.max_entries
            || self.current_memory_bytes > self.max_memory_bytes
        {
            self.collect_garbage();
        }
    }

    /// Record which layers contributed to an already-inserted frame.
    pub fn set_frame_layers(&mut self, frame: u32, layer_indices: &[usize]) {
        let key = (frame, self.effective_version());
        if !layer_indices.is_empty() {
            self.frame_layers
                .insert(key, layer_indices.iter().copied().collect());
        }
    }

    /// Discard stale version entries and LRU evict unaccessed frames when memory budget is exceeded.
    pub fn collect_garbage(&mut self) {
        let current = self.effective_version();
        self.collect_garbage_below(current);

        if self.current_memory_bytes > self.max_memory_bytes || self.entries.len() > self.max_entries {
            let target_memory = (self.max_memory_bytes as f64 * 0.75) as usize;
            let target_entries = (self.max_entries as f64 * 0.75) as usize;

            let mut keys_by_access: Vec<((u32, u64), u64)> = self
                .entries
                .iter()
                .map(|(k, v)| (*k, v.lru_stamp))
                .collect();

            // Sort un-accessed entries first for LRU eviction
            keys_by_access.sort_unstable_by_key(|(_k, accessed)| *accessed);

            for (key, _accessed) in keys_by_access {
                if self.current_memory_bytes <= target_memory && self.entries.len() <= target_entries {
                    break;
                }
                if let Some(removed) = self.entries.remove(&key) {
                    let _ = disk_cache::write_frame_in_namespace(
                        &key,
                        self.disk_namespace,
                        &removed.pixels,
                        removed.width,
                        removed.height,
                    );
                    self.frame_layers.remove(&key);
                    self.current_memory_bytes = self
                        .current_memory_bytes
                        .saturating_sub(removed.pixels.len());
                }
            }
        }
    }

    /// Discard all cache entries whose version is strictly older than `target_version`.
    pub fn collect_garbage_below(&mut self, target_version: u64) {
        let mut evicted: Vec<_> = self
            .entries
            .keys()
            .filter(|(_, version)| *version < target_version)
            .copied()
            .collect();
        evicted.sort_unstable();
        for key in evicted {
            let Some(entry) = self.entries.remove(&key) else {
                continue;
            };
            let _ = disk_cache::write_frame_in_namespace(
                &key,
                self.disk_namespace,
                &entry.pixels,
                entry.width,
                entry.height,
            );
            self.frame_layers.remove(&key);
            self.current_memory_bytes =
                self.current_memory_bytes.saturating_sub(entry.pixels.len());
        }
    }

    /// Discard the entire cache (RAM + disk).
    pub fn invalidate_all(&mut self) {
        self.entries.clear();
        self.frame_layers.clear();
        self.current_memory_bytes = 0;
        disk_cache::clear_namespace(self.disk_namespace);
    }

    /// How many entries are currently held (all versions combined).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// How many entries exist for the current version.
    pub fn current_version_len(&self) -> usize {
        let ver = self.effective_version();
        self.entries.keys().filter(|(_, v)| *v == ver).count()
    }
}

#[cfg(test)]
mod tests;
